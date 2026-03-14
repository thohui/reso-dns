use rusqlite::params;
use serde::Serialize;

use crate::database::{DatabaseError, MetricsDatabasePool};

#[derive(Debug, Serialize)]
pub struct TimelineBucket {
    /// Timestamp of the start of this bucket, in milliseconds since epoch.
    pub ts: i64,
    /// Total number of requests in this bucket.
    pub total: i64,
    /// Total number of blocked requests in this bucket.
    pub blocked: i64,
    /// Total number of cached requests in this bucket.
    pub cached: i64,
    /// Total number of errored requests in this bucket.
    pub errors: i64,
    /// Sum of the duration of all requests in this bucket, in milliseconds
    pub sum_duration: i64,
}

pub struct ClientMetrics {
    /// Timestamp of the start of the bucket, in milliseconds since epoch.
    pub bucket_ts: i64,
    /// Client IP or identifier.
    pub client: String,
    /// Total number of requests in this bucket.
    pub total_count: i64,
    /// Total number of blocked requests in this bucket.
    pub blocked_count: i64,
    /// Total number of cached requests in this bucket.
    pub cached_count: i64,
    /// Total number of errored requests in this bucket.
    pub error_count: i64,
    /// Sum of the duration of all requests in this bucket, in milliseconds.
    pub sum_duration: i64,
}

impl ClientMetrics {
    pub async fn batch_upsert(db: &MetricsDatabasePool, rows: &[Self]) -> Result<(), DatabaseError> {
        if rows.is_empty() {
            return Ok(());
        }

        let owned: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    r.bucket_ts,
                    r.client.clone(),
                    r.total_count,
                    r.blocked_count,
                    r.cached_count,
                    r.error_count,
                    r.sum_duration,
                )
            })
            .collect();

        db.interact(move |c| {
            let tx = c.transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO metrics_by_client (bucket_ts, client, total_count, blocked_count, cached_count, error_count, sum_duration)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(bucket_ts, client) DO UPDATE SET
                         total_count = total_count + excluded.total_count,
                         blocked_count = blocked_count + excluded.blocked_count,
                         cached_count = cached_count + excluded.cached_count,
                         error_count = error_count + excluded.error_count,
                         sum_duration = sum_duration + excluded.sum_duration",
                )?;
                for (bucket_ts, client, total, blocked, cached, errors, duration) in &owned {
                    stmt.execute(params![bucket_ts, client, total, blocked, cached, errors, duration])?;
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
                    "SELECT bucket_ts, client, total_count, blocked_count, cached_count, error_count, sum_duration
                     FROM metrics_by_client
                     WHERE bucket_ts >= ?1 AND bucket_ts < ?2
                     ORDER BY bucket_ts",
                )?;
                let iter = stmt.query_map(params![start_ts, end_ts], |r| {
                    Ok(Self {
                        bucket_ts: r.get(0)?,
                        client: r.get(1)?,
                        total_count: r.get(2)?,
                        blocked_count: r.get(3)?,
                        cached_count: r.get(4)?,
                        error_count: r.get(5)?,
                        sum_duration: r.get(6)?,
                    })
                })?;
                iter.collect()
            })
            .await?)
    }

    pub async fn top_clients(
        db: &MetricsDatabasePool,
        since: i64,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT client, SUM(total_count) as count
                     FROM metrics_by_client
                     WHERE bucket_ts >= ?1
                     GROUP BY client
                     ORDER BY count DESC
                     LIMIT ?2",
                )?;
                let iter = stmt.query_map(params![since, limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
                iter.collect()
            })
            .await?)
    }

    pub async fn timeline(db: &MetricsDatabasePool, since: i64) -> Result<Vec<TimelineBucket>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT bucket_ts, SUM(total_count), SUM(blocked_count), SUM(cached_count), SUM(error_count), SUM(sum_duration)
                     FROM metrics_by_client
                     WHERE bucket_ts >= ?1
                     GROUP BY bucket_ts
                     ORDER BY bucket_ts",
                )?;
                let iter = stmt.query_map(params![since], |r| {
                    Ok(TimelineBucket {
                        ts: r.get(0)?,
                        total: r.get(1)?,
                        blocked: r.get(2)?,
                        cached: r.get(3)?,
                        errors: r.get(4)?,
                        sum_duration: r.get(5)?,
                    })
                })?;
                iter.collect()
            })
            .await?)
    }

    pub fn merge(&mut self, other: &Self) {
        self.total_count += other.total_count;
        self.blocked_count += other.blocked_count;
        self.cached_count += other.cached_count;
        self.error_count += other.error_count;
        self.sum_duration += other.sum_duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_metrics_test_db;

    fn make_client_metrics(
        bucket_ts: i64,
        client: &str,
        total: i64,
        blocked: i64,
        cached: i64,
        errors: i64,
        duration: i64,
    ) -> ClientMetrics {
        ClientMetrics {
            bucket_ts,
            client: client.to_string(),
            total_count: total,
            blocked_count: blocked,
            cached_count: cached,
            error_count: errors,
            sum_duration: duration,
        }
    }

    #[tokio::test]
    async fn batch_upsert_accumulates_on_conflict() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![make_client_metrics(1000, "192.168.1.1", 10, 2, 3, 1, 500)];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::list_range(&db.conn, 0, 2000).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].total_count, 20);
        assert_eq!(result[0].blocked_count, 4);
        assert_eq!(result[0].cached_count, 6);
        assert_eq!(result[0].error_count, 2);
        assert_eq!(result[0].sum_duration, 1000);
    }

    #[tokio::test]
    async fn list_range_filters_by_timestamp() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 1, 0, 0, 0, 10),
            make_client_metrics(2000, "a", 1, 0, 0, 0, 10),
            make_client_metrics(3000, "a", 1, 0, 0, 0, 10),
        ];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::list_range(&db.conn, 1500, 2500).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bucket_ts, 2000);
    }

    #[tokio::test]
    async fn top_clients_returns_ordered_by_count() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "low", 5, 0, 0, 0, 10),
            make_client_metrics(1000, "high", 20, 0, 0, 0, 10),
            make_client_metrics(1000, "mid", 10, 0, 0, 0, 10),
        ];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::top_clients(&db.conn, 0, 10).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "high");
        assert_eq!(result[0].1, 20);
        assert_eq!(result[1].0, "mid");
        assert_eq!(result[2].0, "low");
    }

    #[tokio::test]
    async fn top_clients_aggregates_across_buckets() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 10, 0, 0, 0, 10),
            make_client_metrics(2000, "a", 15, 0, 0, 0, 10),
        ];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::top_clients(&db.conn, 0, 10).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 25);
    }

    #[tokio::test]
    async fn top_clients_respects_since_filter() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 100, 0, 0, 0, 10),
            make_client_metrics(2000, "a", 5, 0, 0, 0, 10),
        ];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::top_clients(&db.conn, 1500, 10).await.unwrap();
        assert_eq!(result[0].1, 5);
    }

    #[tokio::test]
    async fn timeline_groups_by_bucket() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 10, 2, 3, 1, 100),
            make_client_metrics(1000, "b", 5, 1, 0, 0, 50),
            make_client_metrics(2000, "a", 3, 0, 1, 0, 30),
        ];
        ClientMetrics::batch_upsert(&db.conn, &rows).await.unwrap();

        let result = ClientMetrics::timeline(&db.conn, 0).await.unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].ts, 1000);
        assert_eq!(result[0].total, 15);
        assert_eq!(result[0].blocked, 3);
        assert_eq!(result[0].cached, 3);
        assert_eq!(result[0].errors, 1);
        assert_eq!(result[0].sum_duration, 150);

        assert_eq!(result[1].ts, 2000);
        assert_eq!(result[1].total, 3);
    }
}
