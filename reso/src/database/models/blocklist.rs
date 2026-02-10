use chrono::Utc;
use serde::Serialize;
use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BlockedDomain {
    pub domain: String,
    pub created_at: i64,
}

impl BlockedDomain {
    pub fn new(domain: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            domain,
            created_at: now,
        }
    }
}

impl BlockedDomain {
    pub async fn insert(self, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = conn.conn().await;
        conn.call(move |c| {
            c.execute(
                "INSERT INTO blocklist (domain, created_at) VALUES (?1, ?2)",
                params![self.domain.as_str(), self.created_at],
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
                    SELECT domain, 
                    created_at 
                    FROM blocklist 
                    ORDER BY created_at
                    LIMIT ?1 OFFSET ?2
                    "#,
                )?;
                let iter = stmt.query_map(params![limit, offset], |r| {
                    let domain = r.get(0)?;
                    let created_at = r.get(1)?;
                    Ok(BlockedDomain { domain, created_at })
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
                    SELECT domain, 
                    created_at 
                    FROM blocklist 
                    ORDER BY created_at
                    "#,
                )?;
                let iter = stmt.query_map([], |r| {
                    let domain = r.get(0)?;
                    let created_at = r.get(1)?;
                    Ok(BlockedDomain { domain, created_at })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        Ok(domains)
    }

    pub async fn row_count(conn: &DatabaseConnection) -> anyhow::Result<usize> {
        let conn = conn.conn().await;
        Ok(conn
            .call(|c| c.query_row("SELECT COUNT(*) FROM blocklist", [], |r| r.get(0)))
            .await?)
    }
}
