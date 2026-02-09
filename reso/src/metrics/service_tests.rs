#[cfg(test)]
mod tests {
    use super::super::service::{LiveStats, MetricsService};
    use super::super::event::{QueryLogEvent, ErrorLogEvent};
    use reso_context::RequestType;
    use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};
    use reso_resolver::ResolveErrorType;
    use std::sync::Arc;
    use tempfile::tempdir;
    use crate::database::connect;

    fn create_test_query_event() -> QueryLogEvent {
        QueryLogEvent {
            ts_ms: 1000,
            transport: RequestType::UDP,
            client: "127.0.0.1".to_string(),
            qname: DomainName::from_user("example.com").unwrap(),
            qtype: RecordType::A,
            rcode: DnsResponseCode::NoError,
            dur_ms: 50,
            cache_hit: false,
            blocked: false,
        }
    }

    fn create_test_error_event() -> ErrorLogEvent {
        ErrorLogEvent {
            ts_ms: 2000,
            transport: RequestType::TCP,
            client: "192.168.1.1".to_string(),
            message: "Test error".to_string(),
            r#type: ResolveErrorType::Timeout,
            dur_ms: 100,
            qname: Some("fail.example.com".to_string()),
            qtype: Some(RecordType::A as i64),
        }
    }

    #[test]
    fn test_live_stats_apply_event() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let event = create_test_query_event();
        stats.apply_event(&event);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.blocked, 0);
        assert_eq!(stats.cached, 0);
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.sum_duration, 50);
    }

    #[test]
    fn test_live_stats_apply_blocked_event() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let mut event = create_test_query_event();
        event.blocked = true;

        stats.apply_event(&event);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.blocked, 1);
        assert_eq!(stats.cached, 0);
    }

    #[test]
    fn test_live_stats_apply_cached_event() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let mut event = create_test_query_event();
        event.cache_hit = true;

        stats.apply_event(&event);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.cached, 1);
        assert_eq!(stats.blocked, 0);
    }

    #[test]
    fn test_live_stats_apply_error() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let error = create_test_error_event();
        stats.apply_error(&error);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.sum_duration, 100);
    }

    #[test]
    fn test_live_stats_multiple_events() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        for i in 0..10 {
            let mut event = create_test_query_event();
            event.dur_ms = i * 10;
            event.blocked = i % 2 == 0;
            event.cache_hit = i % 3 == 0;
            stats.apply_event(&event);
        }

        assert_eq!(stats.total, 10);
        assert_eq!(stats.blocked, 5);
        assert_eq!(stats.cached, 4);
        assert_eq!(stats.sum_duration, 450);
    }

    #[test]
    fn test_live_stats_mixed_events_and_errors() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let event = create_test_query_event();
        stats.apply_event(&event);

        let error = create_test_error_event();
        stats.apply_error(&error);

        assert_eq!(stats.total, 2);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.sum_duration, 150);
    }

    #[tokio::test]
    async fn test_metrics_service_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Arc::new(connect(db_path.to_str().unwrap()).await.unwrap());

        let (handle, stats, service) = MetricsService::new(conn, 100);

        let live = stats.live().await;
        assert_eq!(live.total, 0);
        assert_eq!(live.errors, 0);
        assert_eq!(live.blocked, 0);
        assert_eq!(live.cached, 0);
    }

    #[test]
    fn test_live_stats_clone() {
        let stats1 = LiveStats {
            total: 10,
            blocked: 2,
            cached: 3,
            errors: 1,
            sum_duration: 500,
            live_since: 123456,
        };

        let stats2 = stats1.clone();

        assert_eq!(stats1.total, stats2.total);
        assert_eq!(stats1.blocked, stats2.blocked);
        assert_eq!(stats1.cached, stats2.cached);
        assert_eq!(stats1.errors, stats2.errors);
        assert_eq!(stats1.sum_duration, stats2.sum_duration);
        assert_eq!(stats1.live_since, stats2.live_since);
    }

    #[test]
    fn test_live_stats_serialization() {
        let stats = LiveStats {
            total: 100,
            blocked: 10,
            cached: 20,
            errors: 5,
            sum_duration: 50000,
            live_since: 1234567890,
        };

        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total"], 100);
        assert_eq!(json["blocked"], 10);
        assert_eq!(json["cached"], 20);
        assert_eq!(json["errors"], 5);
        assert_eq!(json["sum_duration"], 50000);
        assert_eq!(json["live_since"], 1234567890);
    }

    #[test]
    fn test_live_stats_both_blocked_and_cached() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let mut event = create_test_query_event();
        event.blocked = true;
        event.cache_hit = true;

        stats.apply_event(&event);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.blocked, 1);
        assert_eq!(stats.cached, 1);
    }

    #[test]
    fn test_live_stats_duration_accumulation() {
        let mut stats = LiveStats {
            total: 0,
            blocked: 0,
            cached: 0,
            errors: 0,
            sum_duration: 0,
            live_since: 0,
        };

        let mut event = create_test_query_event();
        event.dur_ms = 100;
        stats.apply_event(&event);

        event.dur_ms = 200;
        stats.apply_event(&event);

        let error = create_test_error_event();
        stats.apply_error(&error);

        assert_eq!(stats.sum_duration, 400);
    }
}