#[cfg(test)]
mod tests {
    use super::super::service::BlocklistService;
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

    #[tokio::test]
    async fn test_blocklist_service_new() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        assert!(!service.is_blocked("example.com"));
    }

    #[tokio::test]
    async fn test_add_domain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        let result = service.add_domain("blocked.com").await;
        assert!(result.is_ok());

        assert!(service.is_blocked("blocked.com"));
    }

    #[tokio::test]
    async fn test_add_subdomain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        service.add_domain("example.com").await.unwrap();

        assert!(service.is_blocked("example.com"));
        assert!(service.is_blocked("sub.example.com"));
        assert!(service.is_blocked("deep.sub.example.com"));
    }

    #[tokio::test]
    async fn test_remove_domain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        service.add_domain("blocked.com").await.unwrap();
        assert!(service.is_blocked("blocked.com"));

        let result = service.remove_domain("blocked.com").await;
        assert!(result.is_ok());

        assert!(!service.is_blocked("blocked.com"));
    }

    #[tokio::test]
    async fn test_multiple_domains() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        let domains = vec!["bad1.com", "bad2.com", "bad3.com"];
        for domain in &domains {
            service.add_domain(domain).await.unwrap();
        }

        for domain in &domains {
            assert!(service.is_blocked(domain));
        }

        assert!(!service.is_blocked("good.com"));
    }

    #[tokio::test]
    async fn test_add_invalid_domain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        let result = service.add_domain("invalid domain!@#").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_domain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        let result = service.remove_domain("nonexistent.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_matcher_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        {
            let conn = Arc::new(connect(db_path.to_str().unwrap()).await.unwrap());
            run_migrations(&conn).await.unwrap();
            let service = BlocklistService::new(conn);

            service.add_domain("persistent.com").await.unwrap();
            assert!(service.is_blocked("persistent.com"));
        }

        {
            let conn = Arc::new(connect(db_path.to_str().unwrap()).await.unwrap());
            let service = BlocklistService::new(conn);
            service.load_matcher().await.unwrap();

            assert!(service.is_blocked("persistent.com"));
        }
    }

    #[tokio::test]
    async fn test_is_blocked_case_insensitive() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        service.add_domain("blocked.com").await.unwrap();

        assert!(service.is_blocked("blocked.com"));
        assert!(service.is_blocked("BLOCKED.COM"));
        assert!(service.is_blocked("Blocked.Com"));
    }

    #[tokio::test]
    async fn test_is_blocked_with_trailing_dot() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        service.add_domain("blocked.com").await.unwrap();

        assert!(service.is_blocked("blocked.com"));
        assert!(service.is_blocked("blocked.com."));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let conn = setup_test_db().await;
        let service = Arc::new(BlocklistService::new(conn));

        let mut handles = vec![];
        for i in 0..10 {
            let service_clone = service.clone();
            let handle = tokio::spawn(async move {
                let domain = format!("domain{}.com", i);
                service_clone.add_domain(&domain).await.unwrap();
                assert!(service_clone.is_blocked(&domain));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_empty_blocklist() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        assert!(!service.is_blocked("anything.com"));
        assert!(!service.is_blocked("example.org"));
    }

    #[tokio::test]
    async fn test_add_duplicate_domain() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn);

        service.add_domain("duplicate.com").await.unwrap();
        let result = service.add_domain("duplicate.com").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_blocklist_reload_after_modification() {
        let conn = setup_test_db().await;
        let service = BlocklistService::new(conn.clone());

        service.add_domain("before.com").await.unwrap();
        assert!(service.is_blocked("before.com"));

        service.add_domain("after.com").await.unwrap();
        assert!(service.is_blocked("after.com"));
        assert!(service.is_blocked("before.com"));
    }
}