#[cfg(test)]
mod tests {
    use super::super::query_log::DnsQueryLog;
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

    fn create_test_query_log() -> DnsQueryLog {
        DnsQueryLog {
            ts_ms: 1234567890,
            transport: 0,
            client: "192.168.1.1".to_string(),
            qname: "example.com".to_string(),
            qtype: 1,
            rcode: 0,
            blocked: false,
            cache_hit: false,
            dur_ms: 42,
        }
    }

    #[tokio::test]
    async fn test_dns_query_log_insert() {
        let conn = setup_test_db().await;
        let log = create_test_query_log();

        let result = log.insert(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_query_log_batch_insert() {
        let conn = setup_test_db().await;

        let logs: Vec<DnsQueryLog> = (0..10)
            .map(|i| DnsQueryLog {
                ts_ms: 1000 + i,
                transport: 0,
                client: format!("192.168.1.{}", i),
                qname: format!("domain{}.com", i),
                qtype: 1,
                rcode: 0,
                blocked: false,
                cache_hit: false,
                dur_ms: i,
            })
            .collect();

        let result = DnsQueryLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_query_log_batch_insert_empty() {
        let conn = setup_test_db().await;
        let logs: Vec<DnsQueryLog> = vec![];

        let result = DnsQueryLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_query_log_list() {
        let conn = setup_test_db().await;

        for i in 0..5 {
            let log = DnsQueryLog {
                ts_ms: 1000 + i,
                ..create_test_query_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_dns_query_log_list_ordered_by_timestamp() {
        let conn = setup_test_db().await;

        let timestamps = vec![3000, 1000, 2000, 5000, 4000];
        for ts in timestamps {
            let log = DnsQueryLog {
                ts_ms: ts,
                ..create_test_query_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);

        assert_eq!(logs[0].ts_ms, 5000);
        assert_eq!(logs[1].ts_ms, 4000);
        assert_eq!(logs[2].ts_ms, 3000);
        assert_eq!(logs[3].ts_ms, 2000);
        assert_eq!(logs[4].ts_ms, 1000);
    }

    #[tokio::test]
    async fn test_dns_query_log_list_pagination() {
        let conn = setup_test_db().await;

        for i in 0..20 {
            let log = DnsQueryLog {
                ts_ms: 1000 + i,
                ..create_test_query_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let page1 = DnsQueryLog::list(&conn, 5, 0).await.unwrap();
        assert_eq!(page1.len(), 5);

        let page2 = DnsQueryLog::list(&conn, 5, 5).await.unwrap();
        assert_eq!(page2.len(), 5);

        assert_ne!(page1[0].ts_ms, page2[0].ts_ms);
    }

    #[tokio::test]
    async fn test_dns_query_log_blocked() {
        let conn = setup_test_db().await;

        let log = DnsQueryLog {
            blocked: true,
            rcode: 3,
            ..create_test_query_log()
        };

        log.insert(&conn).await.unwrap();

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].blocked);
    }

    #[tokio::test]
    async fn test_dns_query_log_cache_hit() {
        let conn = setup_test_db().await;

        let log = DnsQueryLog {
            cache_hit: true,
            dur_ms: 1,
            ..create_test_query_log()
        };

        log.insert(&conn).await.unwrap();

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].cache_hit);
    }

    #[tokio::test]
    async fn test_dns_query_log_various_qtypes() {
        let conn = setup_test_db().await;

        let qtypes = vec![1, 28, 5, 15, 16];
        for qtype in qtypes {
            let log = DnsQueryLog {
                qtype,
                ..create_test_query_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 5);
    }

    #[tokio::test]
    async fn test_dns_query_log_various_rcodes() {
        let conn = setup_test_db().await;

        let rcodes = vec![0, 1, 2, 3, 4, 5];
        for rcode in rcodes {
            let log = DnsQueryLog {
                rcode,
                ..create_test_query_log()
            };
            log.insert(&conn).await.unwrap();
        }

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 6);
    }

    #[tokio::test]
    async fn test_dns_query_log_transport_types() {
        let conn = setup_test_db().await;

        let log_udp = DnsQueryLog {
            transport: 0,
            ..create_test_query_log()
        };
        let log_tcp = DnsQueryLog {
            transport: 1,
            ..create_test_query_log()
        };

        log_udp.insert(&conn).await.unwrap();
        log_tcp.insert(&conn).await.unwrap();

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_dns_query_log_clone() {
        let log1 = create_test_query_log();
        let log2 = log1.clone();

        assert_eq!(log1.ts_ms, log2.ts_ms);
        assert_eq!(log1.client, log2.client);
        assert_eq!(log1.qname, log2.qname);
    }

    #[tokio::test]
    async fn test_delete_before() {
        let conn = setup_test_db().await;

        let old_log = DnsQueryLog {
            ts_ms: 1000,
            ..create_test_query_log()
        };
        let new_log = DnsQueryLog {
            ts_ms: 5000,
            ..create_test_query_log()
        };

        old_log.insert(&conn).await.unwrap();
        new_log.insert(&conn).await.unwrap();

        super::super::query_log::delete_before(&conn, 3000).await.unwrap();

        let logs = DnsQueryLog::list(&conn, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].ts_ms, 5000);
    }

    #[tokio::test]
    async fn test_batch_insert_large() {
        let conn = setup_test_db().await;

        let logs: Vec<DnsQueryLog> = (0..1000)
            .map(|i| DnsQueryLog {
                ts_ms: 1000 + i,
                qname: format!("domain{}.com", i),
                ..create_test_query_log()
            })
            .collect();

        let result = DnsQueryLog::batch_insert(&conn, &logs).await;
        assert!(result.is_ok());
    }
}