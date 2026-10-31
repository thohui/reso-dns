mod zone;

use std::{net::IpAddr, sync::Arc};

use arc_swap::ArcSwap;
use reso_dns::{ClassType, DnsRecord, RecordType, domain_name::DomainName, message::DnsRecordData};

use crate::database::{
    CoreDatabasePool,
    models::{
        MatchType,
        local_record::{self, LocalRecord},
    },
};

use super::ServiceError;

use zone::LocalZone;
pub use zone::{LocalLookup, ResolvedRecord};

pub struct LocalRecordService {
    zone: ArcSwap<LocalZone>,
    connection: Arc<CoreDatabasePool>,
}

/// Supported record types for local records.
const SUPPORTED_TYPES: &[RecordType] = &[RecordType::A, RecordType::AAAA, RecordType::CNAME];

fn parse_record_type(rtype: u16) -> Result<RecordType, ServiceError> {
    let rt = RecordType::from(rtype);
    if SUPPORTED_TYPES.contains(&rt) {
        Ok(rt)
    } else {
        Err(ServiceError::BadRequest("Unsupported record type".into()))
    }
}

impl LocalRecordService {
    pub async fn initialize(connection: Arc<CoreDatabasePool>) -> anyhow::Result<Self> {
        let service = Self {
            zone: ArcSwap::new(Arc::new(LocalZone::default())),
            connection,
        };
        service.reload().await?;
        Ok(service)
    }

    pub async fn add_record(
        &self,
        name: &str,
        match_type: MatchType,
        record_type: u16,
        value: &str,
        ttl: u32,
    ) -> Result<(), ServiceError> {
        if !matches!(match_type, MatchType::Exact | MatchType::Wildcard) {
            return Err(ServiceError::BadRequest(
                "Local records only support exact and wildcard matching".into(),
            ));
        }
        if name.contains('*') {
            return Err(ServiceError::BadRequest(
                "Domain must not contain wildcards, use the wildcard match type instead".into(),
            ));
        }

        let rtype = parse_record_type(record_type)?;
        let resolved = parse_value(name, rtype, value)?;
        if resolved.record.name.is_root() {
            return Err(ServiceError::BadRequest("Domain must not be the root".into()));
        }
        self.check_cname_conflict(&resolved.record.name, match_type, rtype)
            .await?;
        let record = LocalRecord::new(
            resolved.record.name.to_string(),
            match_type,
            record_type,
            value.to_string(),
            ttl,
        );
        local_record::insert(&self.connection, record).await.map_err(|e| {
            if e.is_unique_constraint_violation() {
                ServiceError::Conflict("A record with the same name and type already exists.".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;
        self.reload().await?;
        Ok(())
    }

    // a cname can't share its name with other records, not even another cname (RFC 1034 3.6.2)
    // nas.home CNAME x + nas.home A 10.0.0.2 -> conflict
    async fn check_cname_conflict(
        &self,
        name: &DomainName,
        match_type: MatchType,
        rtype: RecordType,
    ) -> Result<(), ServiceError> {
        let all = local_record::list_all(&self.connection).await?;

        // backward compatibility: older rows may be stored as typed, e.g. NAS.Home.
        let mut existing_types = all
            .iter()
            .filter(|r| r.match_type == match_type)
            .filter(|r| DomainName::from_user(&r.name).is_ok_and(|n| n == *name))
            .map(|r| RecordType::from(r.record_type));

        let conflict = if rtype == RecordType::CNAME {
            // a cname needs the name to itself
            existing_types.next().is_some()
        } else {
            // anything else can't join a cname
            existing_types.any(|t| t == RecordType::CNAME)
        };

        if conflict {
            return Err(ServiceError::Conflict(
                "A CNAME record cannot share its name with other records.".into(),
            ));
        }
        Ok(())
    }

    pub async fn remove_record(&self, id: i64) -> Result<(), ServiceError> {
        let changed = local_record::delete_by_id(&self.connection, id).await?;
        if !changed {
            return Err(ServiceError::NotFound("Record not found".into()));
        }
        self.reload().await?;
        Ok(())
    }

    pub async fn toggle_record(&self, id: i64) -> Result<(), ServiceError> {
        let changed = local_record::toggle(&self.connection, id).await?;

        if !changed {
            return Err(ServiceError::NotFound("Record not found".into()));
        }

        self.reload().await?;
        Ok(())
    }

    pub fn lookup(&self, qname: &DomainName, qtype: RecordType) -> LocalLookup {
        self.zone.load().lookup(qname, qtype)
    }

    async fn reload(&self) -> Result<(), ServiceError> {
        let all = local_record::list_all(&self.connection).await?;
        let mut zone = LocalZone::default();

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

            match record.match_type {
                MatchType::Exact => zone.insert_exact(resolved),
                MatchType::Wildcard => zone.insert_wildcard(resolved),
                // Rejected by the API and the table constraint.
                MatchType::Domain => {
                    tracing::warn!("skipping local record id={} with domain match type", record.id);
                }
            }
        }

        self.zone.store(Arc::new(zone));
        Ok(())
    }
}

fn parse_value(name: &str, rtype: RecordType, value: &str) -> Result<ResolvedRecord, ServiceError> {
    let domain = DomainName::from_user(name).map_err(|_| ServiceError::BadRequest("Invalid domain format".into()))?;

    let data = match rtype {
        RecordType::A => {
            let ip: IpAddr = value
                .parse()
                .map_err(|_| ServiceError::BadRequest("Invalid IP address format".into()))?;
            match ip {
                IpAddr::V4(v4) => DnsRecordData::Ipv4(v4),
                _ => {
                    return Err(ServiceError::BadRequest(
                        "Expected IPv4 address for A record, got IPv6".into(),
                    ));
                }
            }
        }
        RecordType::AAAA => {
            let ip: IpAddr = value
                .parse()
                .map_err(|_| ServiceError::BadRequest("Invalid IP address format".into()))?;
            match ip {
                IpAddr::V6(v6) => DnsRecordData::Ipv6(v6),
                _ => {
                    return Err(ServiceError::BadRequest(
                        "Expected IPv6 address for AAAA record, got IPv4".into(),
                    ));
                }
            }
        }
        RecordType::CNAME => {
            let target =
                DomainName::from_user(value).map_err(|_| ServiceError::BadRequest("Invalid CNAME target".into()))?;
            DnsRecordData::DomainName(target)
        }
        _ => {
            return Err(ServiceError::BadRequest(
                "Unsupported record type for local records".into(),
            ));
        }
    };

    let dns_record = DnsRecord::new(domain, rtype, ClassType::IN, 300, data);
    Ok(ResolvedRecord { record: dns_record })
}
