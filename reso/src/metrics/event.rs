use reso_context::{ErrorType, RequestType};
use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};

use crate::database::models::activity_log::ActivityLog;

#[derive(Debug, Clone)]
pub struct QueryLogEvent {
    pub ts_ms: i64,
    pub transport: RequestType,
    pub client: String,
    pub qname: DomainName,
    pub qtype: RecordType,
    pub rcode: DnsResponseCode,
    pub dur_ms: u64,
    pub cache_hit: bool,
    pub blocked: bool,
    pub rate_limited: bool,
}

impl QueryLogEvent {
    pub fn into_db_model(self) -> ActivityLog {
        ActivityLog {
            ts_ms: self.ts_ms,
            kind: "query".to_string(),
            id: 0,
            transport: self.transport as i64,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorLogEvent {
    pub ts_ms: i64,
    pub transport: RequestType,
    pub client: String,
    pub message: String,
    pub r#type: ErrorType,
    pub dur_ms: u64,
    pub qname: Option<String>,
    pub qtype: Option<i64>,
}

impl ErrorLogEvent {
    pub fn into_db_model(self) -> ActivityLog {
        ActivityLog {
            ts_ms: self.ts_ms,
            kind: "error".to_string(),
            id: 0,
            transport: self.transport as i64,
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
        }
    }
}
