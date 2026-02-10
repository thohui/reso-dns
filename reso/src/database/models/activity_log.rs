use anyhow::Context;
use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone)]
pub struct ActivityLog {
    pub ts_ms: i64,
    pub kind: String,
    pub source_id: i64,
    pub transport: i64,
    pub client: String,

    pub qname: Option<String>,
    pub qtype: Option<i64>,
    pub rcode: Option<i64>,
    pub blocked: Option<bool>,
    pub cache_hit: Option<bool>,
    pub dur_ms: u64,

    pub error_type: Option<i64>,
    pub error_message: Option<String>,
}

impl ActivityLog {
    pub async fn list(conn: &DatabaseConnection, limit: usize, offset: usize) -> anyhow::Result<Vec<Self>> {
        let conn = conn.conn().await;
        let rows: Vec<ActivityLog> = conn
            .call(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT
                      ts_ms,
                      kind,
                      source_id,
                      transport,
                      client,
                      qname,
                      qtype,
                      rcode,
                      blocked,
                      cache_hit,
                      dur_ms,
                      error_type,
                      error_message
                    FROM activity_log
                    ORDER BY
                      ts_ms DESC,
                      (kind = 'error') DESC,
                      source_id DESC
                    LIMIT ?1 OFFSET ?2
                    "#,
                )?;

                let iter = stmt.query_map(params![limit, offset], |row| {
                    Ok(ActivityLog {
                        ts_ms: row.get(0)?,
                        kind: row.get(1)?,
                        source_id: row.get(2)?,
                        transport: row.get(3)?,
                        client: row.get(4)?,
                        qname: row.get(5)?,
                        qtype: row.get(6)?,
                        rcode: row.get(7)?,
                        blocked: row.get(8)?,
                        cache_hit: row.get(9)?,
                        dur_ms: row.get(10)?,
                        error_type: row.get(11)?,
                        error_message: row.get(12)?,
                    })
                })?;

                iter.collect::<Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("list activity_log rows")?;

        Ok(rows)
    }

    pub async fn row_count(conn: &DatabaseConnection) -> anyhow::Result<usize> {
        let conn = conn.conn().await;

        Ok(conn
            .call(|c| c.query_row("SELECT COUNT(*) FROM activity_log", [], |r| r.get(0)))
            .await?)
    }
}
