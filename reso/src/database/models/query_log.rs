use anyhow::Context;
use rusqlite::params;

use crate::database::MetricsDatabasePool;

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
    pub rate_limited: bool,
}

impl DnsQueryLog {
    pub async fn insert(&self, db: &MetricsDatabasePool) -> anyhow::Result<()> {
        let ts_ms = self.ts_ms;
        let transport = self.transport;
        let client = self.client.clone();
        let qname = self.qname.clone();
        let qtype = self.qtype;
        let rcode = self.rcode;
        let blocked = self.blocked;
        let cache_hit = self.cache_hit;
        let dur_ms = self.dur_ms;
        let rate_limited = self.rate_limited;

        db.interact(move |c| {
            c.execute(
                r#"
            INSERT INTO dns_query_log
              (ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms, rate_limited)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
                params![
                    ts_ms,
                    transport,
                    client,
                    qname,
                    qtype,
                    rcode,
                    blocked,
                    cache_hit,
                    dur_ms,
                    rate_limited
                ],
            )?;
            Ok(())
        })
        .await
        .context("failed to insert DNS query log")?;

        Ok(())
    }

    pub async fn batch_insert(db: &MetricsDatabasePool, rows: &[Self]) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let owned = rows.to_vec();

        db.interact(move |c| {
            let tx = c.transaction()?;

            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO dns_query_log
                      (ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms, rate_limited)
                    VALUES
                      (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
                        r.dur_ms,
                        r.rate_limited
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await
        .context("failed to batch insert DNS query logs")?;

        Ok(())
    }

    pub async fn list(db: &MetricsDatabasePool, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT
                      ts_ms, transport, client, qname, qtype, rcode, blocked, cache_hit, dur_ms, rate_limited
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
                        rate_limited: row.get(9)?,
                    })
                })?;

                iter.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("failed to list DNS query logs")?)
    }

    pub async fn delete_before(db: &MetricsDatabasePool, cutoff_ts_ms: i64) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute("DELETE FROM dns_query_log WHERE ts_ms < ?1", params![cutoff_ts_ms])?;
            Ok(())
        })
        .await
        .context("failed to delete old DNS query logs")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_metrics_test_db;

    fn make_row(ts_ms: i64) -> DnsQueryLog {
        DnsQueryLog {
            ts_ms,
            transport: 1,
            client: "127.0.0.1".to_string(),
            qname: "example.com".to_string(),
            qtype: 1,
            rcode: 0,
            blocked: false,
            cache_hit: true,
            dur_ms: 42,
            rate_limited: false,
        }
    }

    #[tokio::test]
    async fn test_batch_insert_round_trips_fields() {
        let db = setup_metrics_test_db().await.unwrap();
        let row = make_row(1000);

        DnsQueryLog::batch_insert(&db.conn, &[row.clone()]).await.unwrap();

        let results = DnsQueryLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.ts_ms, row.ts_ms);
        assert_eq!(r.transport, row.transport);
        assert_eq!(r.client, row.client);
        assert_eq!(r.qname, row.qname);
        assert_eq!(r.qtype, row.qtype);
        assert_eq!(r.rcode, row.rcode);
        assert_eq!(r.blocked, row.blocked);
        assert_eq!(r.cache_hit, row.cache_hit);
        assert_eq!(r.dur_ms, row.dur_ms);
        assert_eq!(r.rate_limited, row.rate_limited);
    }

    #[tokio::test]
    async fn test_list_ordered_by_ts_desc() {
        let db = setup_metrics_test_db().await.unwrap();

        DnsQueryLog::batch_insert(&db.conn, &[make_row(1000), make_row(3000), make_row(2000)])
            .await
            .unwrap();

        let results = DnsQueryLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results[0].ts_ms, 3000);
        assert_eq!(results[1].ts_ms, 2000);
        assert_eq!(results[2].ts_ms, 1000);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let db = setup_metrics_test_db().await.unwrap();

        let rows: Vec<_> = (1..=5).map(|i| make_row(i * 1000)).collect();
        DnsQueryLog::batch_insert(&db.conn, &rows).await.unwrap();

        let page1 = DnsQueryLog::list(&db.conn, 2, 0).await.unwrap();
        let page2 = DnsQueryLog::list(&db.conn, 2, 2).await.unwrap();
        let page3 = DnsQueryLog::list(&db.conn, 2, 4).await.unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
        // pages should not overlap
        assert_ne!(page1[0].ts_ms, page2[0].ts_ms);
    }

    #[tokio::test]
    async fn test_delete_before_removes_old_rows() {
        let db = setup_metrics_test_db().await.unwrap();

        DnsQueryLog::batch_insert(&db.conn, &[make_row(1000), make_row(2000), make_row(3000)])
            .await
            .unwrap();

        DnsQueryLog::delete_before(&db.conn, 2000).await.unwrap();

        let results = DnsQueryLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.ts_ms >= 2000));
    }
}
