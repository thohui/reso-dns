use reso_context::{ErrorType, RequestType};
use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};

use crate::database::models::activity_log::ActivityLog;

#[derive(Debug, Clone)]
pub struct QueryLogEvent {
    /// Timestamp in milliseconds.
    pub ts_ms: i64,
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
    /// Rate limited
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
    /// Timestamp in milliseconds.
    pub ts_ms: i64,
    /// Transport
    pub transport: RequestType,
    /// Client IP
    pub client: String,
    /// Error message
    pub message: String,
    /// Error Type
    pub r#type: ErrorType,
    /// Duration in milliseconds
    pub dur_ms: u64,
    /// Query name
    pub qname: Option<String>,
    /// Query type
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

#[cfg(test)]
mod tests {
    use super::*;
    use reso_dns::domain_name::DomainName;

    #[test]
    fn test_query_log_event_into_db_model() {
        let event = QueryLogEvent {
            ts_ms: 1234567890,
            transport: RequestType::TCP,
            client: "10.0.0.1".to_string(),
            qname: DomainName::from_ascii("test.com").unwrap(),
            qtype: RecordType::AAAA,
            rcode: DnsResponseCode::NxDomain,
            dur_ms: 100,
            cache_hit: true,
            blocked: false,
            rate_limited: true,
        };

        let m = event.into_db_model();

        assert_eq!(m.kind, "query");
        assert_eq!(m.ts_ms, 1234567890);
        assert_eq!(m.transport, RequestType::TCP as i64);
        assert_eq!(m.qname.as_deref(), Some("test.com"));
        assert_eq!(m.qtype, Some(RecordType::AAAA.to_u16() as i64));
        assert_eq!(m.rcode, Some(DnsResponseCode::NxDomain.to_u16() as i64));
        assert_eq!(m.dur_ms, 100);
        assert_eq!(m.cache_hit, Some(true));
        assert_eq!(m.blocked, Some(false));
        assert_eq!(m.rate_limited, Some(true));
        assert!(m.error_type.is_none());
        assert!(m.error_message.is_none());
    }
}
