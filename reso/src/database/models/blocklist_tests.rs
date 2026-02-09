#[cfg(test)]
mod tests {
    use super::super::blocklist::BlockedDomain;
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

    #[test]
    fn test_blocked_domain_new() {
        let domain = BlockedDomain::new("example.com".to_string());
        assert_eq!(domain.domain, "example.com");
        assert!(domain.created_at > 0);
    }

    #[tokio::test]
    async fn test_blocked_domain_insert() {
        let conn = setup_test_db().await;
        let domain = BlockedDomain::new("blocked.com".to_string());

        let result = domain.insert(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_blocked_domain_delete() {
        let conn = setup_test_db().await;
        let domain = BlockedDomain::new("todelete.com".to_string());

        domain.clone().insert(&conn).await.unwrap();

        let result = domain.delete(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_blocked_domain_list() {
        let conn = setup_test_db().await;

        for i in 0..5 {
            let domain = BlockedDomain::new(format!("domain{}.com", i));
            domain.insert(&conn).await.unwrap();
        }

        let domains = BlockedDomain::list(&conn, 10, 0).await.unwrap();
        assert_eq!(domains.len(), 5);
    }

    #[tokio::test]
    async fn test_blocked_domain_list_pagination() {
        let conn = setup_test_db().await;

        for i in 0..10 {
            let domain = BlockedDomain::new(format!("domain{}.com", i));
            domain.insert(&conn).await.unwrap();
        }

        let page1 = BlockedDomain::list(&conn, 3, 0).await.unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = BlockedDomain::list(&conn, 3, 3).await.unwrap();
        assert_eq!(page2.len(), 3);

        let page3 = BlockedDomain::list(&conn, 3, 6).await.unwrap();
        assert_eq!(page3.len(), 3);

        let page4 = BlockedDomain::list(&conn, 3, 9).await.unwrap();
        assert_eq!(page4.len(), 1);
    }

    #[tokio::test]
    async fn test_blocked_domain_list_all() {
        let conn = setup_test_db().await;

        for i in 0..20 {
            let domain = BlockedDomain::new(format!("domain{}.com", i));
            domain.insert(&conn).await.unwrap();
        }

        let all_domains = BlockedDomain::list_all(&conn).await.unwrap();
        assert_eq!(all_domains.len(), 20);
    }

    #[tokio::test]
    async fn test_blocked_domain_row_count() {
        let conn = setup_test_db().await;

        let count = BlockedDomain::row_count(&conn).await.unwrap();
        assert_eq!(count, 0);

        for i in 0..15 {
            let domain = BlockedDomain::new(format!("domain{}.com", i));
            domain.insert(&conn).await.unwrap();
        }

        let count = BlockedDomain::row_count(&conn).await.unwrap();
        assert_eq!(count, 15);
    }

    #[tokio::test]
    async fn test_blocked_domain_insert_duplicate() {
        let conn = setup_test_db().await;
        let domain1 = BlockedDomain::new("duplicate.com".to_string());
        let domain2 = BlockedDomain::new("duplicate.com".to_string());

        domain1.insert(&conn).await.unwrap();
        let result = domain2.insert(&conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_blocked_domain_delete_nonexistent() {
        let conn = setup_test_db().await;
        let domain = BlockedDomain::new("nonexistent.com".to_string());

        let result = domain.delete(&conn).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_blocked_domain_list_empty() {
        let conn = setup_test_db().await;

        let domains = BlockedDomain::list(&conn, 10, 0).await.unwrap();
        assert_eq!(domains.len(), 0);
    }

    #[tokio::test]
    async fn test_blocked_domain_list_offset_beyond_end() {
        let conn = setup_test_db().await;

        for i in 0..5 {
            let domain = BlockedDomain::new(format!("domain{}.com", i));
            domain.insert(&conn).await.unwrap();
        }

        let domains = BlockedDomain::list(&conn, 10, 100).await.unwrap();
        assert_eq!(domains.len(), 0);
    }

    #[test]
    fn test_blocked_domain_clone() {
        let domain1 = BlockedDomain::new("test.com".to_string());
        let domain2 = domain1.clone();

        assert_eq!(domain1.domain, domain2.domain);
        assert_eq!(domain1.created_at, domain2.created_at);
    }

    #[test]
    fn test_blocked_domain_serialization() {
        let domain = BlockedDomain::new("serialize.com".to_string());
        let json = serde_json::to_value(&domain).unwrap();

        assert_eq!(json["domain"], "serialize.com");
        assert!(json["created_at"].is_number());
    }

    #[test]
    fn test_blocked_domain_equality() {
        let domain1 = BlockedDomain {
            domain: "example.com".to_string(),
            created_at: 123456,
        };
        let domain2 = BlockedDomain {
            domain: "example.com".to_string(),
            created_at: 123456,
        };

        assert_eq!(domain1, domain2);
    }
}