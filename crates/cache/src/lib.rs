use anyhow::anyhow;
use itertools::Itertools;
use moka::{
    Expiry,
    future::{Cache, CacheBuilder},
};
use reso_dns::{
    DnsMessage, DnsRecord, DnsResponseCode,
    message::{ClassType, DnsRecordData, RecordType},
};
use std::{hash::Hash, sync::Arc};
use tokio::{time::Duration, time::Instant};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: Arc<str>,
    pub record_type: RecordType,
    pub class_type: ClassType,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct NegativeCacheKey {
    pub record_type: RecordType,
    pub class_type: ClassType,
}

impl CacheKey {
    /// Construct a `CacheKey` from a `DnsMessage`
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

    /// Convert a `CacheKey` into a `NegativeCacheKey`.
    fn to_negative_cache_key(&self) -> NegativeCacheKey {
        NegativeCacheKey {
            class_type: self.class_type,
            record_type: self.record_type,
        }
    }
}

/// Cache Result
#[derive(Clone, PartialEq, Debug)]
pub enum CacheResult {
    Positive(Vec<DnsRecord>),
    Negative(NegativeResult),
    Miss,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NegativeResult {
    pub kind: NegKind,
    pub soa_record: DnsRecord,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NegKind {
    /// NxDomain,
    NxDomain,
    /// No records available of the requested type.
    NoData,
}

/// Negative entry
#[derive(Clone, PartialEq, Debug)]
pub struct NegativeEntry {
    /// The kind of entry
    kind: NegKind,
    /// Expires at
    expires_at: Instant,
    // SOA cache key
    soa_cache_key: CacheKey,
    /// The expiration time of the SOA cached entry.
    soa_record_expires_at: Instant,
}

/// RRSet
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CacheEntry {
    pub name: Arc<str>,
    pub record_type: RecordType,
    pub records: Arc<[DnsRecord]>,
    pub expires_at: Instant,
}

pub struct DnsMessageCache {
    cache: Cache<CacheKey, CacheEntry>,
    negative_cache: Cache<NegativeCacheKey, NegativeEntry>,
}

impl Default for DnsMessageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsMessageCache {
    pub fn new() -> Self {
        Self {
            cache: CacheBuilder::new(8192).expire_after(CacheExpiry).build(),
            negative_cache: CacheBuilder::new(8192).expire_after(CacheExpiry).build(),
        }
    }

    pub async fn lookup(&self, key: &CacheKey) -> CacheResult {
        let now = Instant::now();

        if let Some(res) = self
            .handle_negative_entry(now, &key.to_negative_cache_key())
            .await
        {
            return res;
        }

        if let Some(res) = self.handle_entry(now, key).await {
            return res;
        }

        CacheResult::Miss
    }

    /// Handle negative entry
    async fn handle_negative_entry(
        &self,
        now: Instant,
        key: &NegativeCacheKey,
    ) -> Option<CacheResult> {
        let entry = match self.negative_cache.get(key).await {
            None => return None,
            Some(entry) => entry,
        };

        let soa_rr = match self.cache.get(&entry.soa_cache_key).await {
            Some(entry) => entry,
            _ => return Some(CacheResult::Miss),
        };

        let mut soa_record = match soa_rr.records.first() {
            Some(record) => {
                if record.record_type == RecordType::SOA {
                    record.clone()
                } else {
                    return Some(CacheResult::Miss);
                }
            }
            None => return Some(CacheResult::Miss),
        };

        // Update the TTL of the record.
        let remaining = entry.expires_at.saturating_duration_since(now).as_secs();
        let updated_ttl = remaining.min(u32::MAX as u64) as u32;
        soa_record.ttl = updated_ttl;

        Some(CacheResult::Negative(NegativeResult {
            kind: entry.kind,
            soa_record,
        }))
    }

    /// Handle Entry.
    async fn handle_entry(&self, now: Instant, key: &CacheKey) -> Option<CacheResult> {
        let entry = match self.cache.get(key).await {
            None => return None,
            Some(entry) => entry,
        };

        let remaining = entry.expires_at.saturating_duration_since(now).as_secs();
        let updated_ttl = remaining.min(u32::MAX as u64) as u32;

        // Mutate the records with their upated ttl.
        let records_with_updated_ttl: Vec<DnsRecord> = entry
            .records
            .iter()
            .cloned()
            .map(|mut r| {
                r.ttl = updated_ttl;
                r
            })
            .collect();

        Some(CacheResult::Positive(records_with_updated_ttl))
    }

    pub async fn insert(&self, query_msg: &DnsMessage, resp_msg: &DnsMessage) {
        // dont cache truncated or non responses.
        if resp_msg.flags.tc || !resp_msg.flags.qr {
            return;
        }

        // Handle negative caching.
        // https://datatracker.ietf.org/doc/html/rfc2308
        if resp_msg.rcode() == DnsResponseCode::NxDomain {
            let soa_record = match resp_msg
                .authority_records()
                .iter()
                .find(|rec| rec.record_type == RecordType::SOA)
            {
                Some(rec) => rec,
                None => return,
            };

            let question = match query_msg.questions().first() {
                Some(q) => q,
                None => return,
            };

            let minimum = match soa_record.data {
                DnsRecordData::Soa { minimum, .. } => minimum,
                _ => return,
            };

            let soa_cache_key = CacheKey {
                class_type: ClassType::IN,
                name: soa_record.name.clone(),
                record_type: soa_record.record_type,
            };

            let soa_rr_expires_at = Instant::now() + Duration::from_secs(soa_record.ttl as u64);

            let soa_rr = CacheEntry {
                name: soa_record.name.clone(),
                expires_at: soa_rr_expires_at,
                record_type: RecordType::SOA,
                records: Arc::from([soa_record.clone()]),
            };

            self.cache.insert(soa_cache_key.clone(), soa_rr).await;

            let key = CacheKey {
                name: question.qname.clone(),
                class_type: question.qclass,
                record_type: question.qtype,
            };

            let ttl = minimum.min(soa_record.ttl);

            let negative_entry = NegativeEntry {
                kind: NegKind::NxDomain,
                expires_at: Instant::now() + Duration::from_secs(ttl as u64),
                soa_cache_key,
                soa_record_expires_at: soa_rr_expires_at,
            };

            self.negative_cache
                .insert(key.to_negative_cache_key(), negative_entry)
                .await;
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
            let entry = CacheEntry {
                name: key.0,
                record_type: cache_key.record_type,
                records: records.into(),
                expires_at,
            };

            tracing::debug!("inserted {:?} to the cache", entry);
            self.cache.insert(cache_key, entry).await;
        }
    }
}

trait Livable {
    fn expires_at(&self) -> Instant;
}

impl Livable for CacheEntry {
    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

impl Livable for NegativeEntry {
    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}
struct CacheExpiry;

impl<K, V> Expiry<K, V> for CacheExpiry
where
    V: Livable,
{
    fn expire_after_create(&self, _: &K, value: &V, _: std::time::Instant) -> Option<Duration> {
        Some(value.expires_at().saturating_duration_since(Instant::now()))
    }
}
