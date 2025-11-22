use anyhow::anyhow;
use itertools::Itertools;
use moka::{
    Expiry,
    future::{Cache, CacheBuilder},
};
use reso_dns::{
    DnsMessage, DnsRecord, DnsResponseCode,
    message::{ClassType, DnsRecordData, RecordType},
    qname::Qname,
};
use std::{
    hash::Hash,
    sync::Arc,
    time::{Duration, Instant},
};

/// Cache key for positive entries.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: Qname,
    pub record_type: RecordType,
    pub class_type: ClassType,
}

/// Cache key for negative entries.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum NegativeCacheKey {
    /// NoData cache key.
    NoData {
        name: Qname,
        qtype: RecordType,
        class_type: ClassType,
    },
    /// NxDomain cache key.
    NxDomain { qname: Qname, class_type: ClassType },
}

impl TryFrom<&DnsMessage> for CacheKey {
    type Error = anyhow::Error;
    fn try_from(message: &DnsMessage) -> Result<Self, Self::Error> {
        message
            .questions()
            .first()
            .map(|q| CacheKey {
                name: q.qname.clone(),
                class_type: q.qclass,
                record_type: q.qtype,
            })
            .ok_or_else(|| anyhow!("no question in message"))
    }
}

/// Cache Result
#[derive(Clone, PartialEq, Debug)]
pub enum CacheResult {
    Positive(Arc<[DnsRecord]>),
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
    pub name: Qname,
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
        Self::new(50_000)
    }
}

impl DnsMessageCache {
    /// Create a new `DnsMessageCache`
    pub fn new(max_entries: u64) -> Self {
        Self {
            cache: CacheBuilder::new(max_entries)
                .initial_capacity(max_entries as usize)
                .expire_after(CacheExpiry)
                .build(),
            negative_cache: CacheBuilder::new(8192).expire_after(CacheExpiry).build(),
        }
    }

    pub async fn lookup(&self, key: &CacheKey) -> CacheResult {
        let now = Instant::now();

        if let Some(res) = self.handle_entry(now, key).await {
            return res;
        }

        if let Some(res) = self.handle_negative_entry(now, key).await {
            return res;
        }

        CacheResult::Miss
    }

    /// Handle negative entry
    async fn handle_negative_entry(&self, now: Instant, key: &CacheKey) -> Option<CacheResult> {
        let nxdomain_key = NegativeCacheKey::NxDomain {
            qname: key.name.clone(),
            class_type: key.class_type,
        };
        let no_data_key = NegativeCacheKey::NoData {
            name: key.name.clone(),
            qtype: key.record_type,
            class_type: key.class_type,
        };

        // QTYPE=ANY cannot have nodata, only NXDOMAIN (or positive).
        let entry = if key.record_type == RecordType::ANY {
            self.negative_cache.get(&nxdomain_key).await?
        } else {
            let (nx, nd) = tokio::join!(
                self.negative_cache.get(&nxdomain_key),
                self.negative_cache.get(&no_data_key),
            );
            match (nx, nd) {
                (Some(e), _) => e,
                (None, Some(e)) => e,
                (None, None) => return None,
            }
        };

        let soa_rr = self.cache.get(&entry.soa_cache_key).await?;

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
        let entry = self.cache.get(key).await?;

        let remaining = entry.expires_at.saturating_duration_since(now).as_secs();
        let updated_ttl = remaining.min(u32::MAX as u64) as u32;

        // TODO: can we avoid the clone and just return a tuple of records and ttl?

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

        Some(CacheResult::Positive(records_with_updated_ttl.into()))
    }

    pub async fn insert(&self, query_msg: &DnsMessage, resp_msg: &DnsMessage) {
        // dont cache truncated or non responses.
        if resp_msg.flags.tc || !resp_msg.flags.qr {
            return;
        }

        // Handle negative caching.
        // https://datatracker.ietf.org/doc/html/rfc2308
        if resp_msg.flags.aa {
            match resp_msg.rcode() {
                Ok(DnsResponseCode::NoError) => {
                    // Check for nodata
                    let is_no_data = resp_msg.answers().is_empty()
                        && resp_msg
                            .authority_records()
                            .iter()
                            .any(|r| r.record_type == RecordType::SOA);

                    if is_no_data {
                        let soa_record = match resp_msg
                            .authority_records()
                            .iter()
                            .find(|r| r.record_type == RecordType::SOA)
                        {
                            Some(r) => r,
                            None => return,
                        };

                        let Some(question) = query_msg.questions().first() else {
                            return;
                        };

                        let DnsRecordData::SOA { minimum, .. } = soa_record.data else {
                            return;
                        };

                        let soa_cache_key = CacheKey {
                            class_type: soa_record.class,
                            name: soa_record.name.clone(),
                            record_type: RecordType::SOA,
                        };
                        let soa_rr_expires_at =
                            Instant::now() + Duration::from_secs(soa_record.ttl as u64);
                        let soa_rr = CacheEntry {
                            name: soa_record.name.clone(),
                            expires_at: soa_rr_expires_at,
                            record_type: RecordType::SOA,
                            records: Arc::from([soa_record.clone()]),
                        };
                        self.cache.insert(soa_cache_key.clone(), soa_rr).await;

                        let ttl = minimum.min(soa_record.ttl) as u64;

                        let negative_entry = NegativeEntry {
                            kind: NegKind::NoData,
                            expires_at: Instant::now() + Duration::from_secs(ttl),
                            soa_cache_key,
                            soa_record_expires_at: soa_rr_expires_at,
                        };

                        let key = NegativeCacheKey::NoData {
                            name: question.qname.clone(),
                            qtype: question.qtype,
                            class_type: question.qclass,
                        };
                        self.negative_cache.insert(key, negative_entry).await;
                    }
                }
                Ok(DnsResponseCode::NxDomain) => {
                    let Some(soa_record) = resp_msg
                        .authority_records()
                        .iter()
                        .find(|rec| rec.record_type == RecordType::SOA)
                    else {
                        return;
                    };

                    let Some(question) = query_msg.questions().first() else {
                        return;
                    };

                    let DnsRecordData::SOA { minimum, .. } = soa_record.data else {
                        return;
                    };

                    let soa_cache_key = CacheKey {
                        class_type: soa_record.class,
                        name: soa_record.name.clone(),
                        record_type: soa_record.record_type,
                    };

                    let soa_rr_expires_at =
                        Instant::now() + Duration::from_secs(soa_record.ttl as u64);

                    let soa_rr = CacheEntry {
                        name: soa_record.name.clone(),
                        expires_at: soa_rr_expires_at,
                        record_type: RecordType::SOA,
                        records: Arc::from([soa_record.clone()]),
                    };

                    self.cache.insert(soa_cache_key.clone(), soa_rr).await;

                    let key = NegativeCacheKey::NxDomain {
                        qname: question.qname.clone(),
                        class_type: question.qclass,
                    };

                    let ttl = minimum.min(soa_record.ttl);

                    let negative_entry = NegativeEntry {
                        kind: NegKind::NxDomain,
                        expires_at: Instant::now() + Duration::from_secs(ttl as u64),
                        soa_cache_key,
                        soa_record_expires_at: soa_rr_expires_at,
                    };

                    self.negative_cache.insert(key, negative_entry).await;
                }
                _ => {}
            }
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

            if matches!(key.2, RecordType::OPT) {
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
