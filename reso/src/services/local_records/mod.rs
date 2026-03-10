use std::{collections::HashMap, net::IpAddr, sync::Arc};

use arc_swap::ArcSwap;
use reso_dns::{ClassType, DnsRecord, RecordType, domain_name::DomainName, message::DnsRecordData};

use crate::database::{CoreDatabasePool, models::local_record::LocalRecord};

/// A resolved local record ready to be served as a DNS answer.
#[derive(Debug, Clone)]
pub struct ResolvedRecord {
    pub record: DnsRecord,
}

type RecordKey = (String, RecordType);

pub struct LocalRecordService {
    records: ArcSwap<HashMap<RecordKey, Vec<ResolvedRecord>>>,
    connection: Arc<CoreDatabasePool>,
}

/// Supported record types for local records.
const SUPPORTED_TYPES: &[RecordType] = &[RecordType::A, RecordType::AAAA, RecordType::CNAME];

fn parse_record_type(rtype: u16) -> anyhow::Result<RecordType> {
    let rt = RecordType::from(rtype);
    if SUPPORTED_TYPES.contains(&rt) {
        Ok(rt)
    } else {
        anyhow::bail!("unsupported record type: {}", rtype)
    }
}

impl LocalRecordService {
    pub async fn initialize(connection: Arc<CoreDatabasePool>) -> anyhow::Result<Self> {
        let service = Self {
            records: ArcSwap::new(Arc::new(HashMap::new())),
            connection,
        };
        service.reload().await?;
        Ok(service)
    }

    pub async fn add_record(&self, name: &str, record_type: u16, value: &str, ttl: u32) -> anyhow::Result<()> {
        let rtype = parse_record_type(record_type)?;
        parse_value(name, rtype, value)?;
        let record = LocalRecord::new(name.to_string(), record_type, value.to_string(), ttl);
        record.insert(&self.connection).await?;
        self.reload().await?;
        Ok(())
    }

    pub async fn remove_record(&self, id: i64) -> anyhow::Result<()> {
        LocalRecord::delete(&self.connection, id).await?;
        self.reload().await?;
        Ok(())
    }

    pub async fn toggle_record(&self, id: i64) -> anyhow::Result<()> {
        LocalRecord::toggle(&self.connection, id).await?;
        self.reload().await?;
        Ok(())
    }

    pub fn lookup(&self, name: &str, qtype: RecordType) -> Option<Vec<ResolvedRecord>> {
        let records = self.records.load();
        let key = (name.to_ascii_lowercase(), qtype);
        records.get(&key).cloned()
    }

    async fn reload(&self) -> anyhow::Result<()> {
        let all = LocalRecord::list_all(&self.connection).await?;
        let mut map: HashMap<RecordKey, Vec<ResolvedRecord>> = HashMap::new();

        for record in all.into_iter().filter(|r| r.enabled) {
            let rtype = RecordType::from(record.record_type);
            let resolved = match parse_value(&record.name, rtype, &record.value) {
                Ok(mut r) => {
                    r.record.ttl = record.ttl;
                    r
                }
                Err(e) => {
                    tracing::warn!("skipping invalid local record id={}: {}", record.id, e);
                    continue;
                }
            };

            let key = (resolved.record.name.to_ascii_lowercase(), resolved.record.record_type);
            map.entry(key).or_default().push(resolved);
        }

        self.records.store(Arc::new(map));
        Ok(())
    }
}

fn parse_value(name: &str, rtype: RecordType, value: &str) -> anyhow::Result<ResolvedRecord> {
    let domain = DomainName::from_user(name)?;

    let data = match rtype {
        RecordType::A => {
            let ip: IpAddr = value.parse()?;
            match ip {
                IpAddr::V4(v4) => DnsRecordData::Ipv4(v4),
                _ => anyhow::bail!("expected IPv4 address for A record, got IPv6"),
            }
        }
        RecordType::AAAA => {
            let ip: IpAddr = value.parse()?;
            match ip {
                IpAddr::V6(v6) => DnsRecordData::Ipv6(v6),
                _ => anyhow::bail!("expected IPv6 address for AAAA record, got IPv4"),
            }
        }
        RecordType::CNAME => {
            let target = DomainName::from_user(value)?;
            DnsRecordData::DomainName(target)
        }
        _ => anyhow::bail!("unsupported record type for local records"),
    };

    let dns_record = DnsRecord::new(domain, rtype, ClassType::IN, 300, data);
    Ok(ResolvedRecord { record: dns_record })
}
