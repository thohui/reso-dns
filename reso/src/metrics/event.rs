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
            qtype: self.qtype.to_u16() as i64,
            rcode: self.rcode.to_u16() as i64,
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
mod tests {
    use super::*;

    #[test]
    fn test_query_log_event_creation() {
        let event = QueryLogEvent {
            ts_ms: 1234567890,
            transport: RequestType::Udp,
            client: "192.168.1.1".to_string(),
            qname: DomainName::from_ascii("example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 50,
            cache_hit: false,
            blocked: false,
        };

        assert_eq!(event.ts_ms, 1234567890);
        assert_eq!(event.transport, RequestType::Udp);
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
            transport: RequestType::Tcp,
            client: "10.0.0.1".to_string(),
            qname: DomainName::from_ascii("test.com").unwrap(),
            qtype: RecordType::AAAA,
            rcode: DnsResponseCode::NxDomain,
            dur_ms: 100,
            cache_hit: true,
            blocked: false,
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 1234567890);
        assert_eq!(db_model.transport, RequestType::Tcp as i64);
        assert_eq!(db_model.client, "10.0.0.1");
        assert_eq!(db_model.qname, "test.com");
        assert_eq!(db_model.qtype, RecordType::AAAA.to_u16() as i64);
        assert_eq!(db_model.rcode, DnsResponseCode::NxDomain.to_u16() as i64);
        assert_eq!(db_model.dur_ms, 100);
        assert!(db_model.cache_hit);
        assert!(!db_model.blocked);
    }

    #[test]
    fn test_query_log_event_with_cache_hit() {
        let event = QueryLogEvent {
            ts_ms: 1000,
            transport: RequestType::Udp,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("cached.example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 5,
            cache_hit: true,
            blocked: false,
        };

        let db_model = event.into_db_model();
        assert!(db_model.cache_hit);
        assert_eq!(db_model.dur_ms, 5); // Should be faster from cache
    }

    #[test]
    fn test_query_log_event_with_blocked() {
        let event = QueryLogEvent {
            ts_ms: 2000,
            transport: RequestType::Udp,
            client: "192.168.1.100".to_string(),
            qname: DomainName::from_ascii("blocked.example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::Refused,
            dur_ms: 10,
            cache_hit: false,
            blocked: true,
        };

        let db_model = event.into_db_model();
        assert!(db_model.blocked);
        assert_eq!(db_model.rcode, DnsResponseCode::Refused.to_u16() as i64);
    }

    #[test]
    fn test_error_log_event_creation() {
        let event = ErrorLogEvent {
            ts_ms: 9876543210,
            transport: RequestType::Tcp,
            client: "172.16.0.1".to_string(),
            message: "Connection timeout".to_string(),
            r#type: ResolveErrorType::Timeout,
            dur_ms: 5000,
            qname: Some("example.com".to_string()),
            qtype: Some(1),
        };

        assert_eq!(event.ts_ms, 9876543210);
        assert_eq!(event.transport, RequestType::Tcp);
        assert_eq!(event.client, "172.16.0.1");
        assert_eq!(event.message, "Connection timeout");
        assert_eq!(event.r#type, ResolveErrorType::Timeout);
        assert_eq!(event.dur_ms, 5000);
        assert_eq!(event.qname, Some("example.com".to_string()));
        assert_eq!(event.qtype, Some(1));
    }

    #[test]
    fn test_error_log_event_into_db_model() {
        let event = ErrorLogEvent {
            ts_ms: 1111111111,
            transport: RequestType::Udp,
            client: "8.8.8.8".to_string(),
            message: "DNS server error".to_string(),
            r#type: ResolveErrorType::ServerFailure,
            dur_ms: 200,
            qname: Some("fail.example.com".to_string()),
            qtype: Some(28),
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 1111111111);
        assert_eq!(db_model.transport, RequestType::Udp as i64);
        assert_eq!(db_model.client, "8.8.8.8");
        assert_eq!(db_model.message, "DNS server error");
        assert_eq!(db_model.r#type, ResolveErrorType::ServerFailure as i64);
        assert_eq!(db_model.dur_ms, 200);
        assert_eq!(db_model.qname, Some("fail.example.com".to_string()));
        assert_eq!(db_model.qtype, Some(28));
    }

    #[test]
    fn test_error_log_event_without_qname() {
        let event = ErrorLogEvent {
            ts_ms: 3000,
            transport: RequestType::Tcp,
            client: "1.1.1.1".to_string(),
            message: "Malformed request".to_string(),
            r#type: ResolveErrorType::InvalidRequest,
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
            transport: RequestType::Udp,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("test.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 10,
            cache_hit: false,
            blocked: false,
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
            transport: RequestType::Tcp,
            client: "192.168.1.1".to_string(),
            message: "Error".to_string(),
            r#type: ResolveErrorType::Timeout,
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
            transport: RequestType::Udp,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("debug.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 15,
            cache_hit: false,
            blocked: false,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("QueryLogEvent"));
    }

    #[test]
    fn test_error_log_event_debug() {
        let event = ErrorLogEvent {
            ts_ms: 8000,
            transport: RequestType::Tcp,
            client: "127.0.0.1".to_string(),
            message: "Debug test".to_string(),
            r#type: ResolveErrorType::Timeout,
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
                transport: RequestType::Udp,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_ascii("test.com").unwrap(),
                qtype: rtype,
                rcode: DnsResponseCode::NoError,
                dur_ms: 10,
                cache_hit: false,
                blocked: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.qtype, rtype.to_u16() as i64);
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
                transport: RequestType::Udp,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_ascii("test.com").unwrap(),
                qtype: RecordType::A,
                rcode: rcode,
                dur_ms: 10,
                cache_hit: false,
                blocked: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.rcode, rcode.to_u16() as i64);
        }
    }

    #[test]
    fn test_error_log_event_with_long_message() {
        let long_message = "a".repeat(1000);
        let event = ErrorLogEvent {
            ts_ms: 9000,
            transport: RequestType::Tcp,
            client: "127.0.0.1".to_string(),
            message: long_message.clone(),
            r#type: ResolveErrorType::Timeout,
            dur_ms: 100,
            qname: None,
            qtype: None,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.message, long_message);
    }

    #[test]
    fn test_query_log_event_with_zero_duration() {
        let event = QueryLogEvent {
            ts_ms: 10000,
            transport: RequestType::Udp,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_ascii("instant.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 0,
            cache_hit: true,
            blocked: false,
        };

        let db_model = event.into_db_model();
        assert_eq!(db_model.dur_ms, 0);
    }
}