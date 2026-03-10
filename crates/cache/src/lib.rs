use anyhow::anyhow;
use itertools::Itertools as _;
use moka::{
    Expiry,
    future::{Cache, CacheBuilder},
};
use reso_dns::{
    DnsMessage, DnsRecord, DnsResponseCode,
    domain_name::DomainName,
    message::{ClassType, DnsRecordData, RecordType},
};
use std::{
    hash::Hash,
    sync::Arc,
    time::{Duration, Instant},
};

/// Cache key for positive entries.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub name: DomainName,
    pub record_type: RecordType,
    pub class_type: ClassType,
    pub do_bit: bool,
}

/// Cache key for negative entries.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum NegativeCacheKey {
    /// NoData cache key.
    NoData {
        name: DomainName,
        qtype: RecordType,
        class_type: ClassType,
        do_bit: bool,
    },
    /// NxDomain cache key.
    NxDomain {
        qname: DomainName,
        class_type: ClassType,
        do_bit: bool,
    },
}

fn has_do_bit(message: &DnsMessage) -> bool {
    message.edns().as_ref().is_some_and(|e| e.do_bit())
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
                do_bit: has_do_bit(message),
            })
            .ok_or_else(|| anyhow!("no question in message"))
    }
}

/// Cache Result
#[derive(Clone, PartialEq, Debug)]
pub enum CacheResult {
    Positive { records: Arc<[DnsRecord]>, ttl: u32 },
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
    pub name: DomainName,
    pub record_type: RecordType,
    pub records: Arc<[DnsRecord]>,
    pub expires_at: Instant,
}

/// Minimum TTL (seconds) applied to all cached entries.
const MIN_TTL_SECS: u32 = 30;
/// Maximum TTL (seconds) applied to all cached entries.
const MAX_TTL_SECS: u32 = 86_400;

/// A RFC 2308 compliant DNS message cache.
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

    /// Lookup a cache entry.
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
            do_bit: key.do_bit,
        };
        let no_data_key = NegativeCacheKey::NoData {
            name: key.name.clone(),
            qtype: key.record_type,
            class_type: key.class_type,
            do_bit: key.do_bit,
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

        Some(CacheResult::Positive {
            records: Arc::clone(&entry.records),
            ttl: updated_ttl,
        })
    }

    pub async fn insert(&self, query_msg: &DnsMessage, resp_msg: &DnsMessage) -> bool {
        // Don't cache truncated or non-responses.
        if resp_msg.flags.truncated || !resp_msg.flags.response {
            return false;
        }

        // Negative caching: trust the upstream recursive resolver regardless of AA bit.
        let neg_kind = match resp_msg.response_code() {
            Ok(DnsResponseCode::NxDomain) => Some(NegKind::NxDomain),
            Ok(DnsResponseCode::NoError)
                if resp_msg.answers().is_empty()
                    && resp_msg
                        .authority_records()
                        .iter()
                        .any(|r| r.record_type == RecordType::SOA) =>
            {
                Some(NegKind::NoData)
            }
            _ => None,
        };

        if let Some(kind) = neg_kind
            && let Some(inserted) = self.insert_negative(query_msg, resp_msg, kind).await
        {
            return inserted;
        }

        let mut inserted = false;
        let mut min_ttl: Option<u32> = None;

        for ((name, class, record_type), records) in resp_msg
            .answers()
            .iter()
            .into_group_map_by(|r| (r.name.clone(), r.class, r.record_type))
        {
            if matches!(record_type, RecordType::OPT) {
                continue;
            }

            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(0);
            if ttl == 0 {
                continue;
            }
            let ttl = ttl.clamp(MIN_TTL_SECS, MAX_TTL_SECS);
            min_ttl = Some(min_ttl.map_or(ttl, |m| m.min(ttl)));

            let cache_key = CacheKey {
                name: name.clone(),
                class_type: class,
                record_type,
                do_bit: has_do_bit(query_msg),
            };

            let expires_at = Instant::now() + Duration::from_secs(ttl.into());
            let entry = CacheEntry {
                name,
                record_type: cache_key.record_type,
                records: records.into_iter().cloned().collect::<Vec<_>>().into(),
                expires_at,
            };

            self.cache.insert(cache_key, entry).await;
            inserted = true;
        }

        // Cache the full answer under the query key so CNAME chains get cache hits.
        if let Ok(query_key) = CacheKey::try_from(query_msg) {
            let answers = resp_msg.answers();
            let is_cname_chain = query_key.record_type != RecordType::ANY
                && answers.first().is_some_and(|r| r.record_type == RecordType::CNAME)
                && answers.iter().any(|r| r.record_type == query_key.record_type);
            let is_positive = matches!(resp_msg.response_code(), Ok(DnsResponseCode::NoError));

            if is_cname_chain && is_positive {
                let cacheable: Vec<_> = answers
                    .iter()
                    .filter(|r| !matches!(r.record_type, RecordType::OPT))
                    .cloned()
                    .collect();
                let ttl = cacheable.iter().map(|r| r.ttl()).min().unwrap_or(0);
                if ttl > 0 {
                    let ttl = ttl.clamp(MIN_TTL_SECS, MAX_TTL_SECS);
                    min_ttl = Some(min_ttl.map_or(ttl, |m| m.min(ttl)));
                    let expires_at = Instant::now() + Duration::from_secs(ttl.into());
                    let entry = CacheEntry {
                        name: query_key.name.clone(),
                        record_type: query_key.record_type,
                        records: cacheable.into(),
                        expires_at,
                    };
                    self.cache.insert(query_key, entry).await;
                    inserted = true;
                }
            }
        }

        if let Some(ttl) = min_ttl {
            let qname = query_msg.questions().first().map(|q| q.qname.as_str()).unwrap_or("?");
            tracing::debug!(qname, ttl, "cached response");
        }

        inserted
    }

    /// Insert a negative cache entry (NxDomain or NoData).
    /// Returns `Some(true)` on success, `Some(false)` on invalid data, `None` if
    /// the response lacks the required SOA record.
    async fn insert_negative(&self, query_msg: &DnsMessage, resp_msg: &DnsMessage, kind: NegKind) -> Option<bool> {
        let soa_record = resp_msg
            .authority_records()
            .iter()
            .find(|r| r.record_type == RecordType::SOA)?;

        let question = query_msg.questions().first()?;

        let DnsRecordData::SOA { minimum, .. } = soa_record.data else {
            return Some(false);
        };

        let do_bit = has_do_bit(query_msg);
        let soa_rr_expires_at = Instant::now() + Duration::from_secs(soa_record.ttl as u64);
        let ttl = minimum.min(soa_record.ttl).clamp(MIN_TTL_SECS, MAX_TTL_SECS) as u64;

        let soa_cache_key = CacheKey {
            class_type: soa_record.class,
            name: soa_record.name.clone(),
            record_type: RecordType::SOA,
            do_bit,
        };

        let soa_rr = CacheEntry {
            name: soa_record.name.clone(),
            expires_at: soa_rr_expires_at,
            record_type: RecordType::SOA,
            records: Arc::from([soa_record.clone()]),
        };

        let neg_key = match &kind {
            NegKind::NxDomain => NegativeCacheKey::NxDomain {
                qname: question.qname.clone(),
                class_type: question.qclass,
                do_bit,
            },
            NegKind::NoData => NegativeCacheKey::NoData {
                name: question.qname.clone(),
                qtype: question.qtype,
                class_type: question.qclass,
                do_bit,
            },
        };

        let negative_entry = NegativeEntry {
            kind,
            expires_at: Instant::now() + Duration::from_secs(ttl),
            soa_cache_key: soa_cache_key.clone(),
            soa_record_expires_at: soa_rr_expires_at,
        };

        tokio::join!(
            self.cache.insert(soa_cache_key, soa_rr),
            self.negative_cache.insert(neg_key, negative_entry)
        );

        Some(true)
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
