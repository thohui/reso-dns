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

    #[test]
    fn test_query_log_event_creation() {
        let event = QueryLogEvent {
            ts_ms: 1234567890,
            transport: RequestType::UDP,
            client: "192.168.1.1".to_string(),
            qname: DomainName::from_ascii("example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 50,
            cache_hit: false,
            blocked: false,
            rate_limited: false,
        };

        assert_eq!(event.ts_ms, 1234567890);
        assert_eq!(event.transport, RequestType::UDP);
        assert_eq!(event.client, "192.168.1.1");
        assert_eq!(event.qname.as_str(), "example.com");
        assert_eq!(event.qtype, RecordType::A);
        assert_eq!(event.rcode, DnsResponseCode::NoError);
        assert_eq!(event.dur_ms, 50);
        assert!(!event.cache_hit);
        assert!(!event.blocked);
    }

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

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 1234567890);
        assert_eq!(db_model.kind, "query");
        assert_eq!(db_model.transport, RequestType::TCP as i64);
        assert_eq!(db_model.client, "10.0.0.1");
        assert_eq!(db_model.qname.as_deref(), Some("test.com"));
        assert_eq!(db_model.qtype, Some(RecordType::AAAA.to_u16() as i64));
        assert_eq!(db_model.rcode, Some(DnsResponseCode::NxDomain.to_u16() as i64));
        assert_eq!(db_model.dur_ms, 100);
        assert_eq!(db_model.cache_hit, Some(true));
        assert_eq!(db_model.blocked, Some(false));
        assert_eq!(db_model.rate_limited, Some(true));
        assert!(db_model.error_type.is_none());
        assert!(db_model.error_message.is_none());
    }

    #[test]
    fn test_query_log_event_with_cache_hit() {
        let event = QueryLogEvent {
            ts_ms: 1000,
            transport: RequestType::UDP,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("cached.example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 5,
            cache_hit: true,
            blocked: false,
            rate_limited: false,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.cache_hit, Some(true));
        assert_eq!(db_model.dur_ms, 5);
    }

    #[test]
    fn test_query_log_event_with_blocked() {
        let event = QueryLogEvent {
            ts_ms: 2000,
            transport: RequestType::UDP,
            client: "192.168.1.100".to_string(),
            qname: DomainName::from_ascii("blocked.example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::Refused,
            dur_ms: 10,
            cache_hit: false,
            blocked: true,
            rate_limited: false,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.blocked, Some(true));
        assert_eq!(db_model.rcode, Some(DnsResponseCode::Refused.to_u16() as i64));
    }

    #[test]
    fn test_error_log_event_creation() {
        let event = ErrorLogEvent {
            ts_ms: 9876543210,
            transport: RequestType::TCP,
            client: "172.16.0.1".to_string(),
            message: "Connection timeout".to_string(),
            r#type: ErrorType::Timeout,
            dur_ms: 5000,
            qname: Some("example.com".to_string()),
            qtype: Some(1),
        };

        assert_eq!(event.ts_ms, 9876543210);
        assert_eq!(event.transport, RequestType::TCP);
        assert_eq!(event.client, "172.16.0.1");
        assert_eq!(event.message, "Connection timeout");
        assert_eq!(event.r#type, ErrorType::Timeout);
        assert_eq!(event.dur_ms, 5000);
        assert_eq!(event.qname, Some("example.com".to_string()));
        assert_eq!(event.qtype, Some(1));
    }

    #[test]
    fn test_error_log_event_into_db_model() {
        let event = ErrorLogEvent {
            ts_ms: 1111111111,
            transport: RequestType::UDP,
            client: "8.8.8.8".to_string(),
            message: "DNS server error".to_string(),
            r#type: ErrorType::InvalidRequest,
            dur_ms: 200,
            qname: Some("fail.example.com".to_string()),
            qtype: Some(28),
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 1111111111);
        assert_eq!(db_model.kind, "error");
        assert_eq!(db_model.transport, RequestType::UDP as i64);
        assert_eq!(db_model.client, "8.8.8.8");
        assert_eq!(db_model.error_message.as_deref(), Some("DNS server error"));
        assert_eq!(db_model.error_type, Some(ErrorType::InvalidRequest as i64));
        assert_eq!(db_model.dur_ms, 200);
        assert_eq!(db_model.qname.as_deref(), Some("fail.example.com"));
        assert_eq!(db_model.qtype, Some(28));
        assert!(db_model.rcode.is_none());
        assert!(db_model.blocked.is_none());
    }

    #[test]
    fn test_error_log_event_without_qname() {
        let event = ErrorLogEvent {
            ts_ms: 3000,
            transport: RequestType::TCP,
            client: "1.1.1.1".to_string(),
            message: "Malformed request".to_string(),
            r#type: ErrorType::InvalidRequest,
            dur_ms: 1,
            qname: None,
            qtype: None,
        };

        let db_model = event.into_db_model();

        assert!(db_model.qname.is_none());
        assert!(db_model.qtype.is_none());
    }

    #[test]
    fn test_query_log_event_clone() {
        let event = QueryLogEvent {
            ts_ms: 5000,
            transport: RequestType::UDP,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("test.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 10,
            cache_hit: false,
            blocked: false,
            rate_limited: false,
        };

        let cloned = event.clone();

        assert_eq!(event.ts_ms, cloned.ts_ms);
        assert_eq!(event.client, cloned.client);
        assert_eq!(event.qname, cloned.qname);
    }

    #[test]
    fn test_error_log_event_clone() {
        let event = ErrorLogEvent {
            ts_ms: 6000,
            transport: RequestType::TCP,
            client: "192.168.1.1".to_string(),
            message: "Error".to_string(),
            r#type: ErrorType::Timeout,
            dur_ms: 100,
            qname: Some("test.com".to_string()),
            qtype: Some(1),
        };

        let cloned = event.clone();

        assert_eq!(event.ts_ms, cloned.ts_ms);
        assert_eq!(event.message, cloned.message);
    }

    #[test]
    fn test_query_log_event_debug() {
        let event = QueryLogEvent {
            ts_ms: 7000,
            transport: RequestType::UDP,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("debug.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 15,
            cache_hit: false,
            blocked: false,
            rate_limited: false,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("QueryLogEvent"));
    }

    #[test]
    fn test_error_log_event_debug() {
        let event = ErrorLogEvent {
            ts_ms: 8000,
            transport: RequestType::TCP,
            client: "127.0.0.1".to_string(),
            message: "Debug test".to_string(),
            r#type: ErrorType::Timeout,
            dur_ms: 50,
            qname: None,
            qtype: None,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ErrorLogEvent"));
    }

    #[test]
    fn test_query_log_various_record_types() {
        let record_types = vec![
            RecordType::A,
            RecordType::AAAA,
            RecordType::CNAME,
            RecordType::MX,
            RecordType::TXT,
        ];

        for rtype in record_types {
            let event = QueryLogEvent {
                ts_ms: 1000,
                transport: RequestType::UDP,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_ascii("test.com").unwrap(),
                qtype: rtype,
                rcode: DnsResponseCode::NoError,
                dur_ms: 10,
                cache_hit: false,
                blocked: false,
                rate_limited: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.qtype, Some(rtype.to_u16() as i64));
        }
    }

    #[test]
    fn test_query_log_various_response_codes() {
        let rcodes = vec![
            DnsResponseCode::NoError,
            DnsResponseCode::FormatError,
            DnsResponseCode::ServerFailure,
            DnsResponseCode::NxDomain,
            DnsResponseCode::Refused,
        ];

        for rcode in rcodes {
            let event = QueryLogEvent {
                ts_ms: 1000,
                transport: RequestType::UDP,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_ascii("test.com").unwrap(),
                qtype: RecordType::A,
                rcode: rcode,
                dur_ms: 10,
                cache_hit: false,
                blocked: false,
                rate_limited: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.rcode, Some(rcode.to_u16() as i64));
        }
    }

    #[test]
    fn test_error_log_event_with_long_message() {
        let long_message = "a".repeat(1000);
        let event = ErrorLogEvent {
            ts_ms: 9000,
            transport: RequestType::TCP,
            client: "127.0.0.1".to_string(),
            message: long_message.clone(),
            r#type: ErrorType::Timeout,
            dur_ms: 100,
            qname: None,
            qtype: None,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.error_message.as_deref(), Some(long_message.as_str()));
    }

    #[test]
    fn test_query_log_event_with_zero_duration() {
        let event = QueryLogEvent {
            ts_ms: 10000,
            transport: RequestType::UDP,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("instant.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 0,
            cache_hit: true,
            blocked: false,
            rate_limited: false,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.dur_ms, 0);
    }
}
