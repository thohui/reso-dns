use anyhow::Context;
use tokio_rusqlite::{params, rusqlite};

use crate::database::DatabaseConnection;

#[derive(Debug, Clone)]
pub struct DnsQueryLog {
    pub ts_ms: i64,
    pub transport: i64,
    pub client: String,
    pub qname: String,
    pub qtype: i64,
    pub rcode: i64,
    pub blocked: bool,
    pub cache_hit: bool,
    pub dur_ms: i64,
}

impl DnsQueryLog {
    pub async fn insert(&self, conn: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = conn.conn().await;

        let ts_ms = self.ts_ms;
        let transport = self.transport;
        let client = self.client.clone();
        let qname = self.qname.clone();
        let qtype = self.qtype;
        let rcode = self.rcode;
        let blocked = self.blocked;
        let cache_hit = self.cache_hit;
        let dur_ms = self.dur_ms;

        conn.call(move |c| -> rusqlite::Result<()> {
            c.execute(
                r#"
            INSERT INTO dns_query_log
              (ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
                params![
                    ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms
                ],
            )?;
            Ok(())
        })
        .await
        .context("insert dns_query_log row")?;

        Ok(())
    }

    pub async fn batch_insert(conn: &DatabaseConnection, rows: &[Self]) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let conn = conn.conn().await;

        #[derive(Clone)]
        struct RowOwned {
            ts_ms: i64,
            transport: i64,
            client: String,
            qname: String,
            qtype: i64,
            rcode: i64,
            blocked: bool,
            cache_hit: bool,
            dur_ms: i64,
        }

        let owned: Vec<RowOwned> = rows
            .iter()
            .map(|r| RowOwned {
                ts_ms: r.ts_ms,
                transport: r.transport,
                client: r.client.clone(),
                qname: r.qname.clone(),
                qtype: r.qtype as i64,
                rcode: r.rcode as i64,
                blocked: r.blocked,
                cache_hit: r.cache_hit,
                dur_ms: r.dur_ms,
            })
            .collect();

        conn.call(move |c| -> rusqlite::Result<()> {
            let tx = c.transaction()?;

            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO dns_query_log
                      (ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms)
                    VALUES
                      (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                )?;

                for r in owned {
                    stmt.execute(params![
                        r.ts_ms,
                        r.transport,
                        r.client,
                        r.qname,
                        r.qtype,
                        r.rcode,
                        r.blocked,
                        r.cache_hit,
                        r.dur_ms
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await
        .context("batch insert dns_query_log rows")?;

        Ok(())
    }

    pub async fn list(conn: &DatabaseConnection, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        let conn = conn.conn().await;

        let items = conn
            .call(move |c| -> rusqlite::Result<Vec<Self>> {
                let mut stmt = c.prepare(
                    r#"
                    SELECT
                      ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms
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
                        qname: row.get(3)?,
                        qtype: row.get(4)?,
                        rcode: row.get(5)?,
                        blocked: row.get(6)?,
                        cache_hit: row.get(7)?,
                        dur_ms: row.get(8)?,
                    })
                })?;

                iter.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("list dns_query_log rows")?;

        Ok(items)
    }
}

pub async fn delete_before(conn: &DatabaseConnection, cutoff_ts_ms: i64) -> anyhow::Result<()> {
    let conn = conn.conn().await;

    conn.call(move |c| -> rusqlite::Result<usize> {
        c.execute("DELETE FROM dns_query_log WHERE ts_ms < ?1", params![cutoff_ts_ms])
    })
    .await
    .context("delete dns_query_log rows")?;

    Ok(())
}

#[cfg(test)]
#[path = "query_log_tests.rs"]
mod query_log_tests;