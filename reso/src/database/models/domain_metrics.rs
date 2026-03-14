use rusqlite::params;

use crate::database::{DatabaseError, MetricsDatabasePool};

pub struct DomainMetrics {
    /// Timestamp of the start of this bucket, in milliseconds since epoch.
    pub bucket_ts: i64,
    /// Queried domain name (qname).
    pub qname: String,
    /// Total number of queries for this domain in this bucket.
    pub total_count: i64,
    /// Total number of blocked queries for this domain in this bucket.
    pub blocked_count: i64,
}

impl DomainMetrics {
    pub async fn batch_upsert(db: &MetricsDatabasePool, rows: &[Self]) -> Result<(), DatabaseError> {
        if rows.is_empty() {
            return Ok(());
        }

        let owned: Vec<_> = rows
            .iter()
            .map(|r| (r.bucket_ts, r.qname.clone(), r.total_count, r.blocked_count))
            .collect();

        db.interact(move |c| {
            let tx = c.transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO metrics_by_domain (bucket_ts, qname, total_count, blocked_count)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(bucket_ts, qname) DO UPDATE SET
                         total_count = total_count + excluded.total_count,
                         blocked_count = blocked_count + excluded.blocked_count",
                )?;
                for (bucket_ts, qname, total, blocked) in &owned {
                    stmt.execute(params![bucket_ts, qname, total, blocked])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn list_range(db: &MetricsDatabasePool, start_ts: i64, end_ts: i64) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT bucket_ts, qname, total_count, blocked_count
                     FROM metrics_by_domain
                     WHERE bucket_ts >= ?1 AND bucket_ts < ?2
                     ORDER BY bucket_ts",
                )?;
                let iter = stmt.query_map(params![start_ts, end_ts], |r| {
                    Ok(Self {
                        bucket_ts: r.get(0)?,
                        qname: r.get(1)?,
                        total_count: r.get(2)?,
                        blocked_count: r.get(3)?,
                    })
                })?;
                iter.collect()
            })
            .await?)
    }

    pub async fn top_domains(
        db: &MetricsDatabasePool,
        since: i64,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT qname, SUM(total_count) as count
                     FROM metrics_by_domain
                     WHERE bucket_ts >= ?1
                     GROUP BY qname
                     ORDER BY count DESC
                     LIMIT ?2",
                )?;
                let iter = stmt.query_map(params![since, limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
                iter.collect()
            })
            .await?)
    }

    pub async fn top_blocked(
        db: &MetricsDatabasePool,
        since: i64,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT qname, SUM(blocked_count) as count
                     FROM metrics_by_domain
                     WHERE bucket_ts >= ?1 AND blocked_count > 0
                     GROUP BY qname
                     ORDER BY count DESC
                     LIMIT ?2",
                )?;
                let iter = stmt.query_map(params![since, limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
                iter.collect()
            })
            .await?)
    }

    pub fn merge(&mut self, other: &Self) {
        self.total_count += other.total_count;
        self.blocked_count += other.blocked_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_metrics_test_db;

    fn make_domain_metrics(bucket_ts: i64, qname: &str, total: i64, blocked: i64) -> DomainMetrics {
        DomainMetrics {
            bucket_ts,
            qname: qname.to_string(),
            total_count: total,
            blocked_count: blocked,
        }
    }

    #[tokio::test]
    async fn batch_upsert_accumulates_on_conflict() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![make_domain_metrics(1000, "example.com", 10, 3)];
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = DomainMetrics::list_range(&db.conn, 0, 2000).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].total_count, 20);
        assert_eq!(result[0].blocked_count, 6);
    }

    #[tokio::test]
    async fn list_range_filters_by_timestamp() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_domain_metrics(1000, "a.com", 1, 0),
            make_domain_metrics(2000, "a.com", 1, 0),
            make_domain_metrics(3000, "a.com", 1, 0),
        ];
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = DomainMetrics::list_range(&db.conn, 1500, 2500).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bucket_ts, 2000);
    }

    #[tokio::test]
    async fn top_domains_returns_ordered_by_count() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_domain_metrics(1000, "low.com", 5, 0),
            make_domain_metrics(1000, "high.com", 20, 0),
            make_domain_metrics(1000, "mid.com", 10, 0),
        ];
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = DomainMetrics::top_domains(&db.conn, 0, 10).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "high.com");
        assert_eq!(result[0].1, 20);
        assert_eq!(result[1].0, "mid.com");
        assert_eq!(result[2].0, "low.com");
    }

    #[tokio::test]
    async fn top_domains_aggregates_across_buckets() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_domain_metrics(1000, "a.com", 10, 0),
            make_domain_metrics(2000, "a.com", 15, 0),
        ];
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = DomainMetrics::top_domains(&db.conn, 0, 10).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 25);
    }

    #[tokio::test]
    async fn top_blocked_returns_only_blocked_domains() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_domain_metrics(1000, "clean.com", 50, 0),
            make_domain_metrics(1000, "blocked.com", 10, 8),
            make_domain_metrics(1000, "some-blocked.com", 5, 2),
        ];
        DomainMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = DomainMetrics::top_blocked(&db.conn, 0, 10).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "blocked.com");
        assert_eq!(result[0].1, 8);
        assert_eq!(result[1].0, "some-blocked.com");
        assert_eq!(result[1].1, 2);
    }
}
