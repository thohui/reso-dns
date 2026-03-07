use anyhow::Context;
use rusqlite::params;

use crate::database::{DatabasePool, MetricsDatabasePool};

#[derive(Debug, Clone)]
pub struct DnsErrorLog {
    pub ts_ms: i64,
    pub transport: i64,
    pub client: String,
    pub message: String,
    pub r#type: i64,
    pub dur_ms: i64,
    pub qname: Option<String>,
    pub qtype: Option<i64>,
}

impl DnsErrorLog {
    pub async fn insert(self, db: &MetricsDatabasePool) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute(
                r#"
            INSERT INTO dns_error_log
              (ts_ms, transport, client, message, type, dur_ms, qname, qtype)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
                params![
                    self.ts_ms,
                    self.transport,
                    self.client,
                    self.message,
                    self.r#type,
                    self.dur_ms,
                    self.qname,
                    self.qtype
                ],
            )?;
            Ok(())
        })
        .await
        .context("failed to insert DNS error log")?;

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
            INSERT INTO dns_error_log
              (ts_ms, transport, client, message, type, dur_ms, qname, qtype)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
                )?;

                for r in owned {
                    stmt.execute(params![
                        r.ts_ms,
                        r.transport,
                        r.client,
                        r.message,
                        r.r#type,
                        r.dur_ms,
                        r.qname,
                        r.qtype
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await
        .context("failed to batch insert DNS error logs")?;

        Ok(())
    }

    pub async fn list(db: &MetricsDatabasePool, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT
                      ts_ms, transport, client, message, type, dur_ms, qname, qtype
                    FROM dns_error_log
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
                        dur_ms: row.get(5)?,
                        qname: row.get(6)?,
                        qtype: row.get(7)?,
                    })
                })?;

                iter.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("failed to list DNS error logs")?)
    }

    pub async fn delete_before(db: &MetricsDatabasePool, cutoff_ts_ms: i64) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute("DELETE FROM dns_error_log WHERE ts_ms < ?1", params![cutoff_ts_ms])?;
            Ok(())
        })
        .await
        .context("failed to delete old DNS error logs")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_metrics_test_db;

    fn make_row(ts_ms: i64) -> DnsErrorLog {
        DnsErrorLog {
            ts_ms,
            transport: 1,
            client: "127.0.0.1".to_string(),
            message: "timeout".to_string(),
            r#type: 2,
            dur_ms: 100,
            qname: Some("example.com".to_string()),
            qtype: Some(1),
        }
    }

    #[tokio::test]
    async fn test_batch_insert_round_trips_fields() {
        let db = setup_metrics_test_db().await.unwrap();
        let row = make_row(1000);

        DnsErrorLog::batch_insert(&db.conn, &[row.clone()]).await.unwrap();

        let results = DnsErrorLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.ts_ms, row.ts_ms);
        assert_eq!(r.transport, row.transport);
        assert_eq!(r.client, row.client);
        assert_eq!(r.message, row.message);
        assert_eq!(r.r#type, row.r#type);
        assert_eq!(r.dur_ms, row.dur_ms);
        assert_eq!(r.qname, row.qname);
        assert_eq!(r.qtype, row.qtype);
    }

    #[tokio::test]
    async fn test_optional_fields_stored_as_null() {
        let db = setup_metrics_test_db().await.unwrap();
        let row = DnsErrorLog {
            qname: None,
            qtype: None,
            ..make_row(1000)
        };

        DnsErrorLog::batch_insert(&db.conn, &[row]).await.unwrap();

        let results = DnsErrorLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results[0].qname, None);
        assert_eq!(results[0].qtype, None);
    }

    #[tokio::test]
    async fn test_list_ordered_by_ts_desc() {
        let db = setup_metrics_test_db().await.unwrap();

        DnsErrorLog::batch_insert(&db.conn, &[make_row(1000), make_row(3000), make_row(2000)])
            .await
            .unwrap();

        let results = DnsErrorLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results[0].ts_ms, 3000);
        assert_eq!(results[1].ts_ms, 2000);
        assert_eq!(results[2].ts_ms, 1000);
    }

    #[tokio::test]
    async fn test_delete_before_removes_old_rows() {
        let db = setup_metrics_test_db().await.unwrap();

        DnsErrorLog::batch_insert(&db.conn, &[make_row(1000), make_row(2000), make_row(3000)])
            .await
            .unwrap();

        DnsErrorLog::delete_before(&db.conn, 2000).await.unwrap();

        let results = DnsErrorLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.ts_ms >= 2000));
    }
}
