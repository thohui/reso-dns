use anyhow::Context;
use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone)]
pub struct DnsErrorLog {
    pub ts_ms: i64,
    pub transport: i64,
    pub client: String,
    pub message: String,
    pub r#type: i64,
}

impl DnsErrorLog {
    pub async fn insert(&self, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = conn.conn().await;

        let ts_ms = self.ts_ms;
        let transport = self.transport;
        let client = self.client.clone();
        let message = self.message.clone();
        let r#type = self.r#type.clone();

        conn.call(move |c| -> rusqlite::Result<()> {
            c.execute(
                r#"
            INSERT INTO dns_error_log
              (ts_ms, transport, client, message, type)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
                params![ts_ms, transport, client, message, r#type],
            )?;
            Ok(())
        })
        .await
        .context("insert dns_error_log row")?;

        Ok(())
    }

    pub async fn batch_insert(conn: &DatabaseConnection, rows: &[Self]) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let conn = conn.conn().await;

        #[derive(Clone)]
        struct RowOwned {
            pub ts_ms: i64,
            pub transport: i64,
            pub client: String,
            pub message: String,
            pub r#type: i64,
        }

        let owned: Vec<RowOwned> = rows
            .iter()
            .map(|r| RowOwned {
                ts_ms: r.ts_ms,
                transport: r.transport,
                client: r.client.clone(),
                message: r.message.clone(),
                r#type: r.r#type.clone(),
            })
            .collect();

        conn.call(move |c| -> rusqlite::Result<()> {
            let tx = c.transaction()?;

            {
                let mut stmt = tx.prepare(
                    r#"
            INSERT INTO dns_error_log
              (ts_ms, transport, client, message, type)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
                )?;

                for r in owned {
                    stmt.execute(params![r.ts_ms, r.transport, r.client, r.message, r.r#type])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await
        .context("batch insert dns_error_log rows")?;

        Ok(())
    }

    pub async fn list(conn: &DatabaseConnection, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        let conn = conn.conn().await;

        let items = conn
            .call(move |c| -> rusqlite::Result<Vec<Self>> {
                let mut stmt = c.prepare(
                    r#"
                    SELECT
                      ts_ms, transport, client, message, type
                    FROM dns_query_log
                    ORDER BY ts_ms DESC
                    LIMIT ?1 OFFSET ?2
                    "#,
                )?;

                let iter = stmt.query_map(params![limit, offset], |row| {
                    Ok(Self {
                        ts_ms: row.get(0)?,
                        transport: row.get(1)?,
                        client: row.get(2)?,
                        message: row.get(3)?,
                        r#type: row.get(4)?,
                    })
                })?;

                iter.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("list dns_error_log rows")?;

        Ok(items)
    }

    pub async fn delete_before(conn: &DatabaseConnection, cutoff_ts_ms: i64) -> anyhow::Result<()> {
        let conn = conn.conn().await;

        conn.call(move |c| -> rusqlite::Result<usize> {
            c.execute("DELETE FROM dns_error_log WHERE ts_ms < ?1", params![cutoff_ts_ms])
        })
        .await
        .context("delete dns_error_log rows")?;

        Ok(())
    }
}
