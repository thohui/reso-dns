#[cfg(test)]
mod tests {
    use super::super::event::{QueryLogEvent, ErrorLogEvent};
    use reso_context::RequestType;
    use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};
    use reso_resolver::ResolveErrorType;

    #[test]
    fn test_query_log_event_to_db_model() {
        let event = QueryLogEvent {
            ts_ms: 1234567890,
            transport: RequestType::UDP,
            client: "192.168.1.1".to_string(),
            qname: DomainName::from_user("example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 42,
            cache_hit: true,
            blocked: false,
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 1234567890);
        assert_eq!(db_model.transport, RequestType::UDP as i64);
        assert_eq!(db_model.client, "192.168.1.1");
        assert_eq!(db_model.qname, "example.com.");
        assert_eq!(db_model.qtype, RecordType::A as i64);
        assert_eq!(db_model.rcode, DnsResponseCode::NoError as i64);
        assert_eq!(db_model.dur_ms, 42);
        assert!(db_model.cache_hit);
        assert!(!db_model.blocked);
    }

    #[test]
    fn test_query_log_event_blocked() {
        let event = QueryLogEvent {
            ts_ms: 1234567890,
            transport: RequestType::TCP,
            client: "10.0.0.1".to_string(),
            qname: DomainName::from_user("blocked.com").unwrap(),
            qtype: RecordType::AAAA,
            rcode: DnsResponseCode::NXDomain,
            dur_ms: 1,
            cache_hit: false,
            blocked: true,
        };

        let db_model = event.into_db_model();

        assert!(db_model.blocked);
        assert!(!db_model.cache_hit);
        assert_eq!(db_model.rcode, DnsResponseCode::NXDomain as i64);
    }

    #[test]
    fn test_error_log_event_to_db_model() {
        let event = ErrorLogEvent {
            ts_ms: 9876543210,
            transport: RequestType::UDP,
            client: "192.168.1.100".to_string(),
            message: "Connection timeout".to_string(),
            r#type: ResolveErrorType::Timeout,
            dur_ms: 5000,
            qname: Some("timeout.example.com".to_string()),
            qtype: Some(RecordType::A as i64),
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.ts_ms, 9876543210);
        assert_eq!(db_model.transport, RequestType::UDP as i64);
        assert_eq!(db_model.client, "192.168.1.100");
        assert_eq!(db_model.message, "Connection timeout");
        assert_eq!(db_model.r#type, ResolveErrorType::Timeout as i64);
        assert_eq!(db_model.dur_ms, 5000);
        assert_eq!(db_model.qname, Some("timeout.example.com".to_string()));
        assert_eq!(db_model.qtype, Some(RecordType::A as i64));
    }

    #[test]
    fn test_error_log_event_without_query_info() {
        let event = ErrorLogEvent {
            ts_ms: 1111111111,
            transport: RequestType::TCP,
            client: "172.16.0.1".to_string(),
            message: "Parse error".to_string(),
            r#type: ResolveErrorType::Parse,
            dur_ms: 0,
            qname: None,
            qtype: None,
        };

        let db_model = event.into_db_model();

        assert_eq!(db_model.message, "Parse error");
        assert_eq!(db_model.r#type, ResolveErrorType::Parse as i64);
        assert!(db_model.qname.is_none());
        assert!(db_model.qtype.is_none());
    }

    #[test]
    fn test_query_log_event_various_record_types() {
        let record_types = vec![
            RecordType::A,
            RecordType::AAAA,
            RecordType::CNAME,
            RecordType::MX,
            RecordType::TXT,
        ];

        for qtype in record_types {
            let event = QueryLogEvent {
                ts_ms: 1000,
                transport: RequestType::UDP,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_user("test.com").unwrap(),
                qtype,
                rcode: DnsResponseCode::NoError,
                dur_ms: 10,
                cache_hit: false,
                blocked: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.qtype, qtype as i64);
        }
    }

    #[test]
    fn test_query_log_event_various_response_codes() {
        let rcodes = vec![
            DnsResponseCode::NoError,
            DnsResponseCode::FormErr,
            DnsResponseCode::ServFail,
            DnsResponseCode::NXDomain,
            DnsResponseCode::NotImp,
            DnsResponseCode::Refused,
        ];

        for rcode in rcodes {
            let event = QueryLogEvent {
                ts_ms: 2000,
                transport: RequestType::UDP,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_user("test.com").unwrap(),
                qtype: RecordType::A,
                rcode,
                dur_ms: 5,
                cache_hit: false,
                blocked: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.rcode, rcode as i64);
        }
    }

    #[test]
    fn test_transport_types() {
        let transports = vec![RequestType::UDP, RequestType::TCP];

        for transport in transports {
            let event = QueryLogEvent {
                ts_ms: 3000,
                transport,
                client: "127.0.0.1".to_string(),
                qname: DomainName::from_user("test.com").unwrap(),
                qtype: RecordType::A,
                rcode: DnsResponseCode::NoError,
                dur_ms: 15,
                cache_hit: false,
                blocked: false,
            };

            let db_model = event.into_db_model();
            assert_eq!(db_model.transport, transport as i64);
        }
    }
}