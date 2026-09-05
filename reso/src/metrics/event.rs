use reso_context::{DnsProtocol, ErrorType};
use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};

use crate::{
    database::models::{activity_log::ActivityLog, domain_rule::DomainRule},
    uuid::EntityId,
};

#[derive(Debug, Clone)]
pub struct QueryLogEvent {
    pub ts_ms: i64,
    pub transport: DnsProtocol,
    pub client: String,
    pub qname: DomainName,
    pub qtype: RecordType,
    pub rcode: DnsResponseCode,
    pub dur_ms: u64,
    pub cache_hit: bool,
    pub blocked: bool,
    pub rule_id: Option<EntityId<DomainRule>>,
    pub rate_limited: bool,
    pub upstream_protocol: Option<DnsProtocol>,
}

impl QueryLogEvent {
    pub fn into_db_model(self) -> ActivityLog {
        ActivityLog {
            ts_ms: self.ts_ms,
            kind: "query".to_string(),
            id: 0,
            transport: i64::from(self.transport),
            client: self.client,
            qname: Some(self.qname.to_string()),
            qtype: Some(self.qtype.to_u16() as i64),
            dur_ms: self.dur_ms as i64,
            rcode: Some(self.rcode.to_u16() as i64),
            blocked: Some(self.blocked),
            cache_hit: Some(self.cache_hit),
            rate_limited: Some(self.rate_limited),
            error_type: None,
            error_message: None,
            rule_id: self.rule_id,
            upstream_protocol: self.upstream_protocol.map(i64::from),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorLogEvent {
    pub ts_ms: i64,
    pub transport: DnsProtocol,
    pub client: String,
    pub message: String,
    pub r#type: ErrorType,
    pub dur_ms: u64,
    pub qname: Option<String>,
    pub qtype: Option<i64>,
    pub upstream_protocol: Option<DnsProtocol>,
}

impl ErrorLogEvent {
    pub fn into_db_model(self) -> ActivityLog {
        ActivityLog {
            ts_ms: self.ts_ms,
            kind: "error".to_string(),
            id: 0,
            transport: i64::from(self.transport),
            client: self.client,
            qname: self.qname,
            qtype: self.qtype,
            dur_ms: self.dur_ms as i64,
            rcode: None,
            blocked: None,
            cache_hit: None,
            rate_limited: None,
            error_type: Some(self.r#type as i64),
            error_message: Some(self.message),
            rule_id: None,
            upstream_protocol: self.upstream_protocol.map(i64::from),
        }
    }
}
