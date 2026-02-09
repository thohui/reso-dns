#[cfg(test)]
mod tests {
    use super::super::error_log::DnsErrorLog;
    use crate::database::{connect, run_migrations};
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn setup_test_db() -> Arc<crate::database::DatabaseConnection> {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Arc::new(connect(db_path.to_str().unwrap()).await.unwrap());
        run_migrations(&conn).await.unwrap();
        conn
    }

    fn create_test_error_log() -> DnsErrorLog {
        DnsErrorLog {
            ts_ms: 1234567890,
            transport: 0,
            client: "192.168.1.1".to_string(),
            message: "Test error".to_string(),
            r#type: 1,
            dur_ms: 100,
            qname: Some("error.example.com".to_string()),
            qtype: Some(1),
        }
    }

    #[tokio::test]
    async fn test_dns_error_log_insert() {
        let conn = setup_test_db().await;
        let log = create_test_error_log();

        let result = log.insert(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_error_log_without_query_info() {
        let conn = setup_test_db().await;

        let log = DnsErrorLog {
            qname: None,
            qtype: None,
            ..create_test_error_log()
        };

        let result = log.insert(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_error_log_batch_insert() {
        let conn = setup_test_db().await;

        let logs: Vec<DnsErrorLog> = (0..10)
            .map(|i| DnsErrorLog {
                ts_ms: 1000 + i,
                client: format!("192.168.1.{}", i),
                message: format!("Error {}", i),
                ..create_test_error_log()
            })
            .collect();

        let result = DnsErrorLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_error_log_batch_insert_empty() {
        let conn = setup_test_db().await;
        let logs: Vec<DnsErrorLog> = vec![];

        let result = DnsErrorLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_error_log_list() {
        let conn = setup_test_db().await;

        for i in 0..5 {
            let log = DnsErrorLog {
                ts_ms: 1000 + i,
                message: format!("Error {}", i),
                ..create_test_error_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_dns_error_log_list_ordered_by_timestamp() {
        let conn = setup_test_db().await;

        let timestamps = vec![3000, 1000, 2000, 5000, 4000];
        for ts in timestamps {
            let log = DnsErrorLog {
                ts_ms: ts,
                ..create_test_error_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);

        assert_eq!(logs[0].ts_ms, 5000);
        assert_eq!(logs[1].ts_ms, 4000);
        assert_eq!(logs[2].ts_ms, 3000);
    }

    #[tokio::test]
    async fn test_dns_error_log_list_pagination() {
        let conn = setup_test_db().await;

        for i in 0..20 {
            let log = DnsErrorLog {
                ts_ms: 1000 + i,
                message: format!("Error {}", i),
                ..create_test_error_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let page1 = DnsErrorLog::list(&conn, 5, 0).await.unwrap();
        assert_eq!(page1.len(), 5);

        let page2 = DnsErrorLog::list(&conn, 5, 5).await.unwrap();
        assert_eq!(page2.len(), 5);
    }

    #[tokio::test]
    async fn test_dns_error_log_delete_before() {
        let conn = setup_test_db().await;

        let old_log = DnsErrorLog {
            ts_ms: 1000,
            message: "Old error".to_string(),
            ..create_test_error_log()
        };
        let new_log = DnsErrorLog {
            ts_ms: 5000,
            message: "New error".to_string(),
            ..create_test_error_log()
        };

        old_log.insert(&conn).await.unwrap();
        new_log.insert(&conn).await.unwrap();

        DnsErrorLog::delete_before(&conn, 3000).await.unwrap();

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].ts_ms, 5000);
    }

    #[tokio::test]
    async fn test_dns_error_log_various_error_types() {
        let conn = setup_test_db().await;

        let error_types = vec![0, 1, 2, 3, 4];
        for error_type in error_types {
            let log = DnsErrorLog {
                r#type: error_type,
                ..create_test_error_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_dns_error_log_transport_types() {
        let conn = setup_test_db().await;

        let log_udp = DnsErrorLog {
            transport: 0,
            ..create_test_error_log()
        };
        let log_tcp = DnsErrorLog {
            transport: 1,
            ..create_test_error_log()
        };

        log_udp.insert(&conn).await.unwrap();
        log_tcp.insert(&conn).await.unwrap();

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_dns_error_log_clone() {
        let log1 = create_test_error_log();
        let log2 = log1.clone();

        assert_eq!(log1.ts_ms, log2.ts_ms);
        assert_eq!(log1.message, log2.message);
        assert_eq!(log1.r#type, log2.r#type);
    }

    #[tokio::test]
    async fn test_dns_error_log_long_message() {
        let conn = setup_test_db().await;

        let log = DnsErrorLog {
            message: "Very long error message ".repeat(100),
            ..create_test_error_log()
        };

        let result = log.insert(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_insert_large() {
        let conn = setup_test_db().await;

        let logs: Vec<DnsErrorLog> = (0..1000)
            .map(|i| DnsErrorLog {
                ts_ms: 1000 + i,
                message: format!("Error {}", i),
                ..create_test_error_log()
            })
            .collect();

        let result = DnsErrorLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_error_log_zero_duration() {
        let conn = setup_test_db().await;

        let log = DnsErrorLog {
            dur_ms: 0,
            ..create_test_error_log()
        };

        log.insert(&conn).await.unwrap();

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs[0].dur_ms, 0);
    }

    #[tokio::test]
    async fn test_dns_error_log_high_duration() {
        let conn = setup_test_db().await;

        let log = DnsErrorLog {
            dur_ms: 999999,
            ..create_test_error_log()
        };

        log.insert(&conn).await.unwrap();

        let logs = DnsErrorLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs[0].dur_ms, 999999);
    }
}