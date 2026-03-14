use std::{collections::HashMap, net::IpAddr, sync::Arc};

use arc_swap::ArcSwap;
use reso_dns::{ClassType, DnsRecord, RecordType, domain_name::DomainName, message::DnsRecordData};

use crate::database::{CoreDatabasePool, models::local_record::LocalRecord};

use super::ServiceError;

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

fn parse_record_type(rtype: u16) -> Result<RecordType, ServiceError> {
    let rt = RecordType::from(rtype);
    if SUPPORTED_TYPES.contains(&rt) {
        Ok(rt)
    } else {
        Err(ServiceError::BadRequest(format!("Unsupported record type: {}", rtype)))
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

    pub async fn add_record(&self, name: &str, record_type: u16, value: &str, ttl: u32) -> Result<(), ServiceError> {
        let rtype = parse_record_type(record_type)?;
        parse_value(name, rtype, value)?;
        let record = LocalRecord::new(name.to_string(), record_type, value.to_string(), ttl);
        record.insert(&self.connection).await.map_err(|e| {
            if e.is_unique_constraint_violation() {
                ServiceError::Conflict("A record with the same name and type already exists.".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;
        self.reload().await?;
        Ok(())
    }

    pub async fn remove_record(&self, id: i64) -> Result<(), ServiceError> {
        LocalRecord::delete(&self.connection, id).await?;
        self.reload().await?;
        Ok(())
    }

    pub async fn toggle_record(&self, id: i64) -> Result<(), ServiceError> {
        let changed = LocalRecord::toggle(&self.connection, id).await?;

        if !changed {
            return Err(ServiceError::NotFound("Record not found".into()));
        }

        self.reload().await?;
        Ok(())
    }

    pub fn lookup(&self, name: &str, qtype: RecordType) -> Option<Vec<ResolvedRecord>> {
        let records = self.records.load();
        let key = (name.to_ascii_lowercase(), qtype);
        records.get(&key).cloned()
    }

    async fn reload(&self) -> Result<(), ServiceError> {
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

fn parse_value(name: &str, rtype: RecordType, value: &str) -> Result<ResolvedRecord, ServiceError> {
    let domain = DomainName::from_user(name).map_err(|e| ServiceError::BadRequest(format!("Invalid domain: {e}")))?;

    let data = match rtype {
        RecordType::A => {
            let ip: IpAddr = value
                .parse()
                .map_err(|e| ServiceError::BadRequest(format!("Invalid IP address: {e}")))?;
            match ip {
                IpAddr::V4(v4) => DnsRecordData::Ipv4(v4),
                _ => {
                    return Err(ServiceError::BadRequest(
                        "Expected IPv4 address for A record, got IPv6.".into(),
                    ));
                }
            }
        }
        RecordType::AAAA => {
            let ip: IpAddr = value
                .parse()
                .map_err(|e| ServiceError::BadRequest(format!("Invalid IP address: {e}")))?;
            match ip {
                IpAddr::V6(v6) => DnsRecordData::Ipv6(v6),
                _ => {
                    return Err(ServiceError::BadRequest(
                        "Expected IPv6 address for AAAA record, got IPv4.".into(),
                    ));
                }
            }
        }
        RecordType::CNAME => {
            let target = DomainName::from_user(value)
                .map_err(|e| ServiceError::BadRequest(format!("Invalid CNAME target: {e}")))?;
            DnsRecordData::DomainName(target)
        }
        _ => {
            return Err(ServiceError::BadRequest(
                "Unsupported record type for local records.".into(),
            ));
        }
    };

    let dns_record = DnsRecord::new(domain, rtype, ClassType::IN, 300, data);
    Ok(ResolvedRecord { record: dns_record })
}
