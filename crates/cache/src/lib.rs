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
    /// CNAME chain from the original answer, so cached hits can replay it (https://datatracker.ietf.org/doc/html/rfc2308#section-6)
    pub answer_records: Arc<[DnsRecord]>,
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
    /// The SOA that came with the denial (https://datatracker.ietf.org/doc/html/rfc2308#section-5)
    soa_record: DnsRecord,
    /// CNAME chain from the answer section; the denial is about the last name in it (https://datatracker.ietf.org/doc/html/rfc2308#section-1)
    chain: Arc<[DnsRecord]>,
}

/// Postitive entry
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
        Self::new(8192)
    }
}

impl DnsMessageCache {
    pub fn new(max_entries: u64) -> Self {
        Self {
            cache: CacheBuilder::new(max_entries).expire_after(CacheExpiry).build(),
            negative_cache: CacheBuilder::new(max_entries).expire_after(CacheExpiry).build(),
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

        let remaining = entry.expires_at.saturating_duration_since(now).as_secs();
        let updated_ttl = remaining.min(u32::MAX as u64) as u32;

        // The 30s floor can leave more time remaining than a record's original TTL, never serve a TTL higher than upstream sent.
        let mut soa_record = entry.soa_record.clone();
        soa_record.ttl = updated_ttl.min(soa_record.ttl);

        let answer_records: Vec<DnsRecord> = entry
            .chain
            .iter()
            .cloned()
            .map(|mut r| {
                r.ttl = updated_ttl.min(r.ttl);
                r
            })
            .collect();

        Some(CacheResult::Negative(NegativeResult {
            kind: entry.kind,
            soa_record,
            answer_records: answer_records.into(),
        }))
    }

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
            DnsResponseCode::NxDomain => Some(NegKind::NxDomain),
            DnsResponseCode::NoError if is_nodata(query_msg, resp_msg) => Some(NegKind::NoData),
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

            let is_positive = matches!(resp_msg.response_code(), DnsResponseCode::NoError);

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

    async fn insert_negative(&self, query_msg: &DnsMessage, resp_msg: &DnsMessage, kind: NegKind) -> Option<bool> {
        let soa_record = resp_msg
            .authority_records()
            .iter()
            .find(|r| r.record_type == RecordType::SOA)?;

        let question = query_msg.questions().first()?;

        let DnsRecordData::SOA { minimum, .. } = soa_record.data else {
            return Some(false);
        };

        let chain: Vec<DnsRecord> = resp_msg
            .answers()
            .iter()
            .filter(|r| r.record_type == RecordType::CNAME)
            .cloned()
            .collect();

        // A decremented SOA TTL of 0 means the negative answer must not be
        // reused (RFC 2308 Section 5), minimum 0 disables negative caching likewise.
        let mut ttl = minimum.min(soa_record.ttl);
        if ttl == 0 {
            return Some(false);
        }
        if let Some(chain_min) = chain.iter().map(|r| r.ttl()).min() {
            if chain_min == 0 {
                return Some(false);
            }
            ttl = ttl.min(chain_min);
        }
        let ttl = ttl.clamp(MIN_TTL_SECS, MAX_TTL_SECS) as u64;

        let do_bit = has_do_bit(query_msg);
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
            soa_record: soa_record.clone(),
            chain: chain.into(),
        };

        self.negative_cache.insert(neg_key, negative_entry).await;

        Some(true)
    }
}

/// Check if a resp is of type NODATA (https://datatracker.ietf.org/doc/html/rfc2308#section-2.2)
fn is_nodata(query_msg: &DnsMessage, resp_msg: &DnsMessage) -> bool {
    let Some(question) = query_msg.questions().first() else {
        return false;
    };

    // ANY can't be NODATA.
    if question.qtype == RecordType::ANY {
        return false;
    }

    // CNAME/RRSIG answers to same record type queries are positive, not a chain.
    if matches!(question.qtype, RecordType::CNAME | RecordType::RRSIG) && !resp_msg.answers().is_empty() {
        return false;
    }

    resp_msg
        .authority_records()
        .iter()
        .any(|r| r.record_type == RecordType::SOA)
        && resp_msg
            .answers()
            .iter()
            .all(|r| matches!(r.record_type, RecordType::CNAME | RecordType::RRSIG))
}

trait Expirable {
    fn expires_at(&self) -> Instant;
}

impl Expirable for CacheEntry {
    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

impl Expirable for NegativeEntry {
    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}
struct CacheExpiry;

impl<K, V> Expiry<K, V> for CacheExpiry
where
    V: Expirable,
{
    fn expire_after_create(&self, _: &K, value: &V, _: std::time::Instant) -> Option<Duration> {
        Some(value.expires_at().saturating_duration_since(Instant::now()))
    }

    fn expire_after_update(&self, _: &K, value: &V, _: std::time::Instant, _: Option<Duration>) -> Option<Duration> {
        Some(value.expires_at().saturating_duration_since(Instant::now()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reso_dns::{DnsFlags, DnsMessageBuilder, DnsOpcode, message::DnsQuestion};

    fn name(s: &str) -> DomainName {
        DomainName::from_ascii(s).unwrap()
    }

    fn query_flags() -> DnsFlags {
        DnsFlags::new(false, DnsOpcode::Query, false, false, true, false, false, false)
    }

    fn response_flags() -> DnsFlags {
        DnsFlags::new(true, DnsOpcode::Query, false, false, true, true, false, false)
    }

    fn question(qname: &str, qtype: RecordType) -> DnsQuestion {
        DnsQuestion {
            qname: name(qname),
            qtype,
            qclass: ClassType::IN,
        }
    }

    fn soa_record(zone: &str, ttl: u32, minimum: u32) -> DnsRecord {
        DnsRecord::new(
            name(zone),
            RecordType::SOA,
            ClassType::IN,
            ttl,
            DnsRecordData::SOA {
                mname: name(&format!("ns1.{zone}")),
                rname: name(&format!("hostmaster.{zone}")),
                serial: 1,
                refresh: 7200,
                retry: 3600,
                expire: 1209600,
                minimum,
            },
        )
    }

    /// NODATA behind a CNAME chain should get cached and replay the chain on hits.
    #[tokio::test]
    async fn cname_chain_nodata_is_cached() {
        let cache = DnsMessageCache::default();

        let query = DnsMessageBuilder::new()
            .with_id(1)
            .with_flags(query_flags())
            .add_question(question("www.example.com", RecordType::AAAA))
            .build();

        let response = DnsMessageBuilder::new()
            .with_id(1)
            .with_flags(response_flags())
            .with_response(DnsResponseCode::NoError)
            .add_question(question("www.example.com", RecordType::AAAA))
            .add_answer(DnsRecord::new(
                name("www.example.com"),
                RecordType::CNAME,
                ClassType::IN,
                300,
                DnsRecordData::DomainName(name("edge.cdn-provider.net")),
            ))
            .add_authority_record(soa_record("cdn-provider.net", 900, 900))
            .build();

        cache.insert(&query, &response).await;

        let key = CacheKey::try_from(&query).unwrap();
        match cache.lookup(&key).await {
            CacheResult::Negative(result) => {
                assert_eq!(result.kind, NegKind::NoData);
                assert_eq!(result.answer_records.len(), 1);
                assert_eq!(result.answer_records[0].record_type, RecordType::CNAME);
            }
            other => panic!("expected NODATA hit, got {other:?}"),
        }
    }

    // The min-TTL floor keeps negative entries alive past short SOA TTLs.
    #[tokio::test]
    async fn negative_entry_ttl_floor_outlives_short_soa() {
        let cache = DnsMessageCache::default();

        let query = DnsMessageBuilder::new()
            .with_id(2)
            .with_flags(query_flags())
            .add_question(question("nonexistent.example.com", RecordType::A))
            .build();

        let response = DnsMessageBuilder::new()
            .with_id(2)
            .with_flags(response_flags())
            .with_response(DnsResponseCode::NxDomain)
            .add_question(question("nonexistent.example.com", RecordType::A))
            .add_authority_record(soa_record("example.com", 1, 3600))
            .build();

        cache.insert(&query, &response).await;

        let key = CacheKey::try_from(&query).unwrap();
        assert!(matches!(cache.lookup(&key).await, CacheResult::Negative(_)));

        tokio::time::sleep(Duration::from_millis(1300)).await;

        assert!(matches!(cache.lookup(&key).await, CacheResult::Negative(_)));
    }
}
