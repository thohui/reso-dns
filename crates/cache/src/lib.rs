use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use itertools::Itertools;
use moka::future::Cache;
use reso_dns::{
    DnsMessage, DnsRecord, DnsResponseCode,
    message::{EdnsOption, RecordType},
};
use std::{hash::Hash, sync::Arc, time::Duration};
use tokio::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: Arc<str>,
    pub record_type: reso_dns::message::RecordType,
    pub class_type: reso_dns::message::ClassType,
}

impl CacheKey {
    pub fn from_message(message: &reso_dns::message::DnsMessage) -> anyhow::Result<Self> {
        if let Some(question) = message.questions().first() {
            Ok(CacheKey {
                name: question.qname.clone(),
                class_type: question.qclass,
                record_type: question.qtype,
            })
        } else {
            Err(anyhow!("no question in message"))
        }
    }
}

/// Cache Result
#[derive(Clone, PartialEq, Debug)]
pub enum CacheResult {
    Positive(Vec<DnsRecord>),
    Negative(NegativeEntry),
    Miss,
}

/// Negative entry
#[derive(Clone, PartialEq, Debug)]
pub struct NegativeEntry {
    response_code: DnsResponseCode,
    expires_at: Instant,
}

/// The entry stored in the cache.
#[derive(Clone, PartialEq, Debug)]
pub enum CacheEntry {
    RRSet(RRSet),
    Negative(NegativeEntry),
}

/// RRSet
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RRSet {
    pub name: Arc<str>,
    pub record_type: RecordType,
    pub records: Arc<[DnsRecord]>,
    pub expires_at: Instant,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DnsResponseBytes(Bytes);

impl DnsResponseBytes {
    pub fn new(bytes: Bytes) -> Self {
        Self(bytes)
    }

    pub fn into_custom_response(self, transaction_id: u16) -> Bytes {
        let mut bytes = BytesMut::from(&self.0[0..]);
        // replace the transaction id in the cached response.
        bytes[0] = (transaction_id >> 8) as u8;
        bytes[1] = (transaction_id & 0xFF) as u8;
        bytes.freeze()
    }
}
pub struct MessageCache {
    cache: Cache<CacheKey, CacheEntry>,
}

impl MessageCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(8192),
        }
    }
    pub async fn lookup(&self, key: &CacheKey) -> CacheResult {
        let now = Instant::now();
        // TODO: add support for negative caching.
        if let Some(CacheEntry::RRSet(entry)) = self.cache.get(key).await {
            if entry.expires_at > now {
                let remaining = entry.expires_at.saturating_duration_since(now).as_secs();
                let updated_ttl = remaining.min(u32::MAX as u64) as u32;
                let records_with_updated_ttl: Vec<DnsRecord> = entry
                    .records
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        r.ttl = updated_ttl;
                        r
                    })
                    .collect();
                return CacheResult::Positive(records_with_updated_ttl);
            } else {
                self.cache.remove(key).await;
            }
        }
        CacheResult::Miss
    }

    pub async fn insert(
        &self,
        query_msg: &DnsMessage,
        resp_msg: &DnsMessage,
    ) -> anyhow::Result<()> {
        // dont cache truncated or non responses.
        if resp_msg.flags.tc || !resp_msg.flags.qr {
            return Ok(());
        }

        // skip if there is an edns cookie (for now)
        let has_cookie = resp_msg
            .edns()
            .as_ref()
            .map(|e| {
                e.options
                    .iter()
                    .any(|opt| matches!(opt, EdnsOption::Cookie(_)))
            })
            .unwrap_or(false);

        if has_cookie {
            return Ok(());
        }

        // TODO: negative caching.
        // need to implement soa parsing first.
        if resp_msg.rcode() == DnsResponseCode::NxDomain {
            if let Some(question) = query_msg.questions().first() {
                // let key = CacheKey {
                //     name: question.qname.clone(),
                //     class_type: question.qclass,
                //     record_type: question.qtype,
                // };
            }
            return Ok(());
        }

        // Group the records by their record types.
        let grouped_records: Vec<_> = resp_msg
            .answers()
            .iter()
            .chunk_by(|r| (r.name.clone(), r.class, r.record_type))
            .into_iter()
            .map(|(key, group)| {
                let records: Vec<_> = group.cloned().collect();
                (key, records)
            })
            .collect();

        for (key, records) in grouped_records {
            // Skip EDNS
            // TODO: we should probably support ENDS later on.
            if key.2 == RecordType::OPT {
                continue;
            }

            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(0);

            // Skip if ttl is negative
            if ttl == 0 {
                continue;
            }

            let cache_key = CacheKey {
                name: key.0.clone(),
                class_type: key.1,
                record_type: key.2,
            };

            let expires_at = Instant::now() + Duration::from_secs(ttl.into());
            let entry = RRSet {
                name: key.0,
                record_type: cache_key.record_type,
                records: records.into(),
                expires_at,
            };

            tracing::debug!("inserted {:?} to the cache", entry);

            self.cache.insert(cache_key, CacheEntry::RRSet(entry)).await;
        }

        Ok(())
    }
}

impl Default for MessageCache {
    fn default() -> Self {
        Self::new()
    }
}
