#[cfg(test)]
mod tests {
    use super::super::activity::{Activity, ActivityKind, ActivityQuery, ActivityError};
    use crate::database::models::activity_log::ActivityLog;

    #[test]
    fn test_activity_from_query_log() {
        let log = ActivityLog {
            ts_ms: 1234567890,
            kind: "query".to_string(),
            source_id: 1,
            transport: 0,
            client: "192.168.1.1".to_string(),
            qname: Some("example.com".to_string()),
            qtype: Some(1),
            rcode: Some(0),
            blocked: Some(false),
            cache_hit: Some(true),
            dur_ms: 42,
            error_type: None,
            error_message: None,
        };

        let activity = Activity::try_from(log).expect("conversion failed");

        assert_eq!(activity.timestamp, 1234567890);
        assert_eq!(activity.transport, 0);
        assert_eq!(activity.client, Some("192.168.1.1".to_string()));
        assert_eq!(activity.duration, 42);
        assert_eq!(activity.qname, Some("example.com".to_string()));
        assert_eq!(activity.qtype, Some(1));

        match activity.kind {
            ActivityKind::Query(query) => {
                assert_eq!(query.source_id, 1);
                assert_eq!(query.rcode, 0);
                assert!(!query.blocked);
                assert!(query.cache_hit);
            }
            _ => panic!("Expected Query kind"),
        }
    }

    #[test]
    fn test_activity_from_error_log() {
        let log = ActivityLog {
            ts_ms: 9876543210,
            kind: "error".to_string(),
            source_id: 2,
            transport: 1,
            client: "10.0.0.1".to_string(),
            qname: Some("fail.example.com".to_string()),
            qtype: Some(28),
            rcode: None,
            blocked: None,
            cache_hit: None,
            dur_ms: 5000,
            error_type: Some(1),
            error_message: Some("Connection timeout".to_string()),
        };

        let activity = Activity::try_from(log).expect("conversion failed");

        assert_eq!(activity.timestamp, 9876543210);
        assert_eq!(activity.transport, 1);
        assert_eq!(activity.client, Some("10.0.0.1".to_string()));
        assert_eq!(activity.duration, 5000);

        match activity.kind {
            ActivityKind::Error(error) => {
                assert_eq!(error.source_id, 2);
                assert_eq!(error.error_type, 1);
                assert_eq!(error.message, "Connection timeout");
            }
            _ => panic!("Expected Error kind"),
        }
    }

    #[test]
    fn test_activity_from_blocked_query() {
        let log = ActivityLog {
            ts_ms: 1111111111,
            kind: "query".to_string(),
            source_id: 3,
            transport: 0,
            client: "172.16.0.1".to_string(),
            qname: Some("blocked.com".to_string()),
            qtype: Some(1),
            rcode: Some(3),
            blocked: Some(true),
            cache_hit: Some(false),
            dur_ms: 1,
            error_type: None,
            error_message: None,
        };

        let activity = Activity::try_from(log).expect("conversion failed");

        match activity.kind {
            ActivityKind::Query(query) => {
                assert!(query.blocked);
                assert!(!query.cache_hit);
                assert_eq!(query.rcode, 3);
            }
            _ => panic!("Expected Query kind"),
        }
    }

    #[test]
    fn test_activity_invalid_kind() {
        let log = ActivityLog {
            ts_ms: 1000,
            kind: "invalid".to_string(),
            source_id: 1,
            transport: 0,
            client: "127.0.0.1".to_string(),
            qname: None,
            qtype: None,
            rcode: None,
            blocked: None,
            cache_hit: None,
            dur_ms: 0,
            error_type: None,
            error_message: None,
        };

        let result = Activity::try_from(log);
        assert!(result.is_err());
    }

    #[test]
    fn test_activity_query_missing_rcode() {
        let log = ActivityLog {
            ts_ms: 1000,
            kind: "query".to_string(),
            source_id: 1,
            transport: 0,
            client: "127.0.0.1".to_string(),
            qname: Some("example.com".to_string()),
            qtype: Some(1),
            rcode: None,
            blocked: Some(false),
            cache_hit: Some(true),
            dur_ms: 10,
            error_type: None,
            error_message: None,
        };

        let result = Activity::try_from(log);
        assert!(result.is_err());
    }

    #[test]
    fn test_activity_error_missing_message() {
        let log = ActivityLog {
            ts_ms: 2000,
            kind: "error".to_string(),
            source_id: 2,
            transport: 1,
            client: "192.168.1.1".to_string(),
            qname: None,
            qtype: None,
            rcode: None,
            blocked: None,
            cache_hit: None,
            dur_ms: 100,
            error_type: Some(1),
            error_message: None,
        };

        let result = Activity::try_from(log);
        assert!(result.is_err());
    }

    #[test]
    fn test_activity_transport_out_of_range() {
        let log = ActivityLog {
            ts_ms: 1000,
            kind: "query".to_string(),
            source_id: 1,
            transport: 256,
            client: "127.0.0.1".to_string(),
            qname: Some("example.com".to_string()),
            qtype: Some(1),
            rcode: Some(0),
            blocked: Some(false),
            cache_hit: Some(true),
            dur_ms: 10,
            error_type: None,
            error_message: None,
        };

        let result = Activity::try_from(log);
        assert!(result.is_err());
    }

    #[test]
    fn test_activity_serialization_query() {
        let activity = Activity {
            timestamp: 123456,
            transport: 0,
            client: Some("127.0.0.1".to_string()),
            duration: 50,
            qname: Some("test.com".to_string()),
            qtype: Some(1),
            kind: ActivityKind::Query(ActivityQuery {
                source_id: 1,
                rcode: 0,
                blocked: false,
                cache_hit: true,
            }),
        };

        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["timestamp"], 123456);
        assert_eq!(json["transport"], 0);
        assert_eq!(json["client"], "127.0.0.1");
        assert_eq!(json["duration"], 50);
        assert_eq!(json["kind"], "query");
        assert!(json["d"].is_object());
    }

    #[test]
    fn test_activity_serialization_error() {
        let activity = Activity {
            timestamp: 654321,
            transport: 1,
            client: Some("10.0.0.1".to_string()),
            duration: 100,
            qname: Some("error.com".to_string()),
            qtype: Some(28),
            kind: ActivityKind::Error(ActivityError {
                source_id: 2,
                error_type: 1,
                message: "Timeout".to_string(),
            }),
        };

        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["kind"], "error");
        assert_eq!(json["d"]["error_type"], 1);
        assert_eq!(json["d"]["message"], "Timeout");
    }

    #[test]
    fn test_activity_various_rcodes() {
        for rcode in 0..6 {
            let log = ActivityLog {
                ts_ms: 1000,
                kind: "query".to_string(),
                source_id: 1,
                transport: 0,
                client: "127.0.0.1".to_string(),
                qname: Some("example.com".to_string()),
                qtype: Some(1),
                rcode: Some(rcode),
                blocked: Some(false),
                cache_hit: Some(false),
                dur_ms: 10,
                error_type: None,
                error_message: None,
            };

            let activity = Activity::try_from(log).expect("conversion failed");
            match activity.kind {
                ActivityKind::Query(query) => {
                    assert_eq!(query.rcode, rcode as u16);
                }
                _ => panic!("Expected Query kind"),
            }
        }
    }
}