use reso_context::RequestType;
use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};
use reso_resolver::ResolveErrorType;

use crate::database::models::{error_log::DnsErrorLog, query_log::DnsQueryLog};

type TsMs = i64;

#[derive(Debug, Clone)]
pub struct QueryLogEvent {
    /// Timestamp in milliseconds.
    pub ts_ms: TsMs,
    /// Transport
    pub transport: RequestType,
    /// Client IP
    pub client: String,
    /// Domain Name
    pub qname: DomainName,
    /// Record Type
    pub qtype: RecordType,
    /// Response code
    pub rcode: DnsResponseCode,
    /// Duration in milliseconds.
    pub dur_ms: u64,
    /// Cache hit
    pub cache_hit: bool,
    /// Blocked
    pub blocked: bool,
}

impl QueryLogEvent {
    pub fn into_db_model(self) -> DnsQueryLog {
        DnsQueryLog {
            ts_ms: self.ts_ms,
            blocked: self.blocked,
            transport: self.transport as i64,
            client: self.client,
            cache_hit: self.cache_hit,
            dur_ms: self.dur_ms as i64,
            qname: self.qname.to_string(),
            qtype: self.qtype as i64,
            rcode: self.rcode as i64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorLogEvent {
    /// Timestamp in milliseconds.
    pub ts_ms: TsMs,
    /// Transport
    pub transport: RequestType,
    /// Client IP
    pub client: String,
    /// Error message
    pub message: String,
    /// Error Type
    pub r#type: ResolveErrorType,
    /// Duration in milliseconds
    pub dur_ms: u64,

    /// Query name
    pub qname: Option<String>,
    /// Query type
    pub qtype: Option<i64>,
}

impl ErrorLogEvent {
    pub fn into_db_model(self) -> DnsErrorLog {
        DnsErrorLog {
            ts_ms: self.ts_ms,
            transport: self.transport as i64,
            client: self.client,
            message: self.message,
            r#type: self.r#type as i64,
            dur_ms: self.dur_ms,
            qname: self.qname,
            qtype: self.qtype,
        }
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;