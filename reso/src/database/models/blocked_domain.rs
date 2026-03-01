use chrono::Utc;
use serde::Serialize;
use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BlockedDomain {
    pub domain: String,
    pub created_at: i64,
    pub enabled: bool,
}

impl BlockedDomain {
    pub fn new(domain: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            domain,
            created_at: now,
            enabled: true,
        }
    }
}

impl BlockedDomain {
    pub async fn insert(self, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = conn.conn().await;
        conn.call(move |c| {
            c.execute(
                "INSERT INTO blocklist (domain, created_at, enabled) VALUES (?1, ?2, ?3)",
                params![self.domain.as_str(), self.created_at, self.enabled],
            )
        })
        .await?;
        Ok(())
    }

    pub async fn delete(self, db: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = db.conn().await;

        conn.call(move |c| c.execute("DELETE FROM blocklist where domain = ?1", params![self.domain]))
            .await?;
        Ok(())
    }

    pub async fn list(conn: &DatabaseConnection, limit: usize, offset: usize) -> anyhow::Result<Vec<Self>> {
        let conn = conn.conn().await;

        let domains: Vec<BlockedDomain> = conn
            .call(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT domain, created_at, enabled
                    FROM blocklist
                    ORDER BY created_at
                    LIMIT ?1 OFFSET ?2
                    "#,
                )?;
                let iter = stmt.query_map(params![limit, offset], |r| {
                    Ok(BlockedDomain {
                        domain: r.get(0)?,
                        created_at: r.get(1)?,
                        enabled: r.get(2)?,
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        Ok(domains)
    }

    pub async fn list_all(conn: &DatabaseConnection) -> anyhow::Result<Vec<Self>> {
        let conn = conn.conn().await;

        let domains: Vec<BlockedDomain> = conn
            .call(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT domain, created_at, enabled
                    FROM blocklist
                    ORDER BY created_at
                    "#,
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(BlockedDomain {
                        domain: r.get(0)?,
                        created_at: r.get(1)?,
                        enabled: r.get(2)?,
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        Ok(domains)
    }

    pub async fn toggle(domain: &str, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let domain = domain.to_string();
        let conn = conn.conn().await;
        conn.call(move |c| {
            c.execute(
                "UPDATE blocklist SET enabled = NOT enabled WHERE domain = ?1",
                params![domain],
            )
        })
        .await?;
        Ok(())
    }

    pub async fn row_count(conn: &DatabaseConnection) -> anyhow::Result<usize> {
        let conn = conn.conn().await;
        Ok(conn
            .call(|c| c.query_row("SELECT COUNT(*) FROM blocklist", [], |r| r.get(0)))
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_test_db;

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_test_db().await.unwrap();
        let domain = BlockedDomain::new("google.com".into());
        domain.clone().insert(&db).await.unwrap();

        let domains = BlockedDomain::list(&db, 10, 0).await.unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0], domain);
    }

    #[tokio::test]
    async fn test_insert_and_list_all() {
        let db = setup_test_db().await.unwrap();
        BlockedDomain::new("a.com".into()).insert(&db).await.unwrap();
        BlockedDomain::new("b.com".into()).insert(&db).await.unwrap();

        let all = BlockedDomain::list_all(&db).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let db = setup_test_db().await.unwrap();
        for i in 0..5 {
            BlockedDomain::new(format!("domain{i}.com")).insert(&db).await.unwrap();
        }

        let page1 = BlockedDomain::list(&db, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = BlockedDomain::list(&db, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = BlockedDomain::list(&db, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_test_db().await.unwrap();
        let domain = BlockedDomain::new("delete-me.com".into());
        domain.clone().insert(&db).await.unwrap();

        assert_eq!(BlockedDomain::row_count(&db).await.unwrap(), 1);

        domain.delete(&db).await.unwrap();
        assert_eq!(BlockedDomain::row_count(&db).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_row_count() {
        let db = setup_test_db().await.unwrap();
        assert_eq!(BlockedDomain::row_count(&db).await.unwrap(), 0);

        BlockedDomain::new("a.com".into()).insert(&db).await.unwrap();
        assert_eq!(BlockedDomain::row_count(&db).await.unwrap(), 1);

        BlockedDomain::new("b.com".into()).insert(&db).await.unwrap();
        assert_eq!(BlockedDomain::row_count(&db).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_toggle() {
        let db = setup_test_db().await.unwrap();
        BlockedDomain::new("toggle.com".into()).insert(&db).await.unwrap();

        let before = BlockedDomain::list(&db, 1, 0).await.unwrap();
        assert!(before[0].enabled);

        BlockedDomain::toggle("toggle.com", &db).await.unwrap();

        let after = BlockedDomain::list(&db, 1, 0).await.unwrap();
        assert!(!after[0].enabled);

        BlockedDomain::toggle("toggle.com", &db).await.unwrap();

        let restored = BlockedDomain::list(&db, 1, 0).await.unwrap();
        assert!(restored[0].enabled);
    }

    #[tokio::test]
    async fn test_enabled_persisted() {
        let db = setup_test_db().await.unwrap();
        BlockedDomain::new("test.com".into()).insert(&db).await.unwrap();

        let domains = BlockedDomain::list_all(&db).await.unwrap();
        assert!(domains[0].enabled);

        BlockedDomain::toggle("test.com", &db).await.unwrap();

        let domains = BlockedDomain::list_all(&db).await.unwrap();
        assert!(!domains[0].enabled);
    }

    #[tokio::test]
    async fn test_list_empty() {
        let db = setup_test_db().await.unwrap();
        let domains = BlockedDomain::list(&db, 10, 0).await.unwrap();
        assert!(domains.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_insert_fails() {
        let db = setup_test_db().await.unwrap();
        BlockedDomain::new("dup.com".into()).insert(&db).await.unwrap();

        let result = BlockedDomain::new("dup.com".into()).insert(&db).await;
        assert!(result.is_err());
    }
}
