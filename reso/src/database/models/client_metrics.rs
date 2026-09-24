use rusqlite::{params, types::Value};
use serde::Serialize;

use crate::database::{DatabaseError, MetricsDatabasePool, query::WhereBuilder};

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
    /// Sum of the duration of all requests in this bucket in milliseconds
    pub sum_duration: i64,
    /// How wide this bucket is in milliseconds.
    pub bucket_duration: i64,
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
    pub fn empty(bucket_ts: i64, client: String) -> Self {
        Self {
            bucket_ts,
            client,
            total_count: 0,
            blocked_count: 0,
            cached_count: 0,
            error_count: 0,
            sum_duration: 0,
        }
    }
}

pub struct GenericMetrics {
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

/// Sum metrics since `since`, for a single client or across all clients when `client` is None.
pub async fn metrics_totals(
    db: &MetricsDatabasePool,
    client: Option<String>,
    since: i64,
) -> Result<GenericMetrics, DatabaseError> {
    db.interact(move |c| {

        let mut b = WhereBuilder::new(1);
        if let Some(client) = client {
            b.eq("client", Value::Text(client));
        }

        let (where_clause, filter_params) = b.build();

        let sql = format!(
            "SELECT COALESCE(SUM(total_count), 0), COALESCE(SUM(blocked_count), 0), COALESCE(SUM(cached_count), 0), COALESCE(SUM(error_count), 0), COALESCE(SUM(sum_duration), 0)
            FROM metrics_by_client
            WHERE bucket_ts >= ?1 {where_clause}"
        );
        let params = std::iter::once(Value::Integer(since)).chain(filter_params);
        c.query_row(&sql, rusqlite::params_from_iter(params), |r| {
            Ok(GenericMetrics {
                total_count: r.get(0)?,
                blocked_count: r.get(1)?,
                cached_count: r.get(2)?,
                error_count: r.get(3)?,
                sum_duration: r.get(4)?,
            })
        })
    })
    .await
}

/// Batch upsert client metrics
/// on conflict, the counts and duration will be accumulated.
pub async fn batch_upsert(db: &MetricsDatabasePool, rows: Vec<ClientMetrics>) -> Result<(), DatabaseError> {
    if rows.is_empty() {
        return Ok(());
    }

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
            for r in &rows {
                stmt.execute(params![
                    r.bucket_ts,
                    r.client,
                    r.total_count,
                    r.blocked_count,
                    r.cached_count,
                    r.error_count,
                    r.sum_duration
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    })
    .await?;
    Ok(())
}

/// List top clients by total count since the given timestamp, ordered by count descending.
pub async fn top_clients(
    db: &MetricsDatabasePool,
    since: i64,
    limit: i64,
) -> Result<Vec<(String, i64)>, DatabaseError> {
    db.interact(move |c| {
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
    .await
}

/// Get timeline of total counts, blocked counts, cached counts, error counts, and sum duration, grouped into `bucket_width` wide buckets.
pub async fn timeline(
    db: &MetricsDatabasePool,
    since: i64,
    bucket_width: i64,
) -> Result<Vec<TimelineBucket>, DatabaseError> {
    db.interact(move |c| {
        // round each bucket down to its slot start so buckets in one slot group together
        let mut stmt = c.prepare(&format!(
            "SELECT (bucket_ts / {bucket_width}) * {bucket_width} AS slot_ts,
                    SUM(total_count), SUM(blocked_count), SUM(cached_count), SUM(error_count), SUM(sum_duration)
             FROM metrics_by_client
             WHERE bucket_ts >= ?1
             GROUP BY slot_ts
             ORDER BY slot_ts",
        ))?;
        let iter = stmt.query_map(params![since], |r| {
            Ok(TimelineBucket {
                ts: r.get(0)?,
                total: r.get(1)?,
                blocked: r.get(2)?,
                cached: r.get(3)?,
                errors: r.get(4)?,
                sum_duration: r.get(5)?,
                bucket_duration: bucket_width,
            })
        })?;
        iter.collect()
    })
    .await
}

/// Compress old metric buckets into larger ones to save space.
/// `cutoff` is a unix timestamp in ms, all buckets with a timestamp older than the cutoff will be compressed.
/// `bucket_ms` is the target bucket width in milliseconds.
pub async fn compress_before(db: &MetricsDatabasePool, cutoff: i64, bucket_ms: i64) -> Result<(), DatabaseError> {
    db.interact(move |c| {
        // find all rows older than the cutoff that aren't already aligned to bucket_ms and sum
        // them into bucket_ms-aligned buckets. rows whose bucket_ts is already divisible by bucket_ms are already compressed, so we skip those.

        // (bucket_ts / bucket_ms) * bucket_ms floors the timestamp to the start of the bucket.
        let rolled: Vec<ClientMetrics> = {
            let mut q = c.prepare(&format!(
                "SELECT (bucket_ts / {bucket_ms}) * {bucket_ms} AS rolled_ts, client,
                        SUM(total_count), SUM(blocked_count), SUM(cached_count),
                        SUM(error_count), SUM(sum_duration)
                 FROM metrics_by_client
                 WHERE bucket_ts < ?1
                   AND bucket_ts % {bucket_ms} != 0
                 GROUP BY rolled_ts, client",
            ))?;
            q.query_map(params![cutoff], |r| {
                Ok(ClientMetrics {
                    bucket_ts: r.get(0)?,
                    client: r.get(1)?,
                    total_count: r.get(2)?,
                    blocked_count: r.get(3)?,
                    cached_count: r.get(4)?,
                    error_count: r.get(5)?,
                    sum_duration: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?
        };

        if rolled.is_empty() {
            return Ok(());
        }

        let tx = c.transaction()?;
        {
            let mut upsert = tx.prepare(
                "INSERT INTO metrics_by_client
                     (bucket_ts, client, total_count, blocked_count, cached_count, error_count, sum_duration)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(bucket_ts, client) DO UPDATE SET
                     total_count   = total_count   + excluded.total_count,
                     blocked_count = blocked_count + excluded.blocked_count,
                     cached_count  = cached_count  + excluded.cached_count,
                     error_count   = error_count   + excluded.error_count,
                     sum_duration  = sum_duration  + excluded.sum_duration",
            )?;
            for row in &rolled {
                upsert.execute(params![
                    row.bucket_ts,
                    row.client,
                    row.total_count,
                    row.blocked_count,
                    row.cached_count,
                    row.error_count,
                    row.sum_duration,
                ])?;
            }
        }
        tx.execute(
            &format!("DELETE FROM metrics_by_client WHERE bucket_ts < ?1 AND bucket_ts % {bucket_ms} != 0"),
            params![cutoff],
        )?;
        tx.commit()?;
        Ok(())
    })
    .await?;
    Ok(())
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

    async fn list_range_client_metrics(
        db: &MetricsDatabasePool,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<ClientMetrics>, DatabaseError> {
        db.interact(move |c| {
            let mut stmt = c.prepare(
                "SELECT bucket_ts, client, total_count, blocked_count, cached_count, error_count, sum_duration
                     FROM metrics_by_client
                     WHERE bucket_ts >= ?1 AND bucket_ts < ?2
                     ORDER BY bucket_ts",
            )?;
            let iter = stmt.query_map(params![start_ts, end_ts], |r| {
                Ok(ClientMetrics {
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
        .await
    }

    #[tokio::test]
    async fn batch_upsert_accumulates_on_conflict() {
        let db = setup_metrics_test_db().await.unwrap();
        batch_upsert(
            &db.conn,
            vec![make_client_metrics(1000, "192.168.1.1", 10, 2, 3, 1, 500)],
        )
        .await
        .unwrap();
        batch_upsert(
            &db.conn,
            vec![make_client_metrics(1000, "192.168.1.1", 10, 2, 3, 1, 500)],
        )
        .await
        .unwrap();

        let result = list_range_client_metrics(&db.conn, 0, 2000).await.unwrap();
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
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = list_range_client_metrics(&db.conn, 1500, 2500).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bucket_ts, 2000);
    }

    #[tokio::test]
    async fn metrics_totals_client_sums_only_that_client() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 10, 2, 3, 1, 100),
            make_client_metrics(2000, "a", 5, 1, 1, 0, 50),
            make_client_metrics(1000, "b", 99, 9, 9, 9, 999),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = metrics_totals(&db.conn, Some("a".to_string()), 0).await.unwrap();
        assert_eq!(result.total_count, 15);
        assert_eq!(result.blocked_count, 3);
        assert_eq!(result.cached_count, 4);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.sum_duration, 150);
    }

    #[tokio::test]
    async fn metrics_totals_client_respects_since_filter() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 100, 0, 0, 0, 10),
            make_client_metrics(2000, "a", 5, 0, 0, 0, 10),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = metrics_totals(&db.conn, Some("a".to_string()), 1500).await.unwrap();
        assert_eq!(result.total_count, 5);
    }

    #[tokio::test]
    async fn metrics_totals_client_returns_zeros_for_unknown_client() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![make_client_metrics(1000, "a", 10, 0, 0, 0, 100)];
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = metrics_totals(&db.conn, Some("unknown".to_string()), 0).await.unwrap();
        assert_eq!(result.total_count, 0);
        assert_eq!(result.sum_duration, 0);
    }

    #[tokio::test]
    async fn metrics_totals_sums_across_clients() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "a", 10, 2, 3, 1, 100),
            make_client_metrics(2000, "a", 5, 1, 1, 0, 50),
            make_client_metrics(1000, "b", 7, 0, 2, 2, 70),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = metrics_totals(&db.conn, None, 0).await.unwrap();
        assert_eq!(result.total_count, 22);
        assert_eq!(result.blocked_count, 3);
        assert_eq!(result.cached_count, 6);
        assert_eq!(result.error_count, 3);
        assert_eq!(result.sum_duration, 220);
    }

    #[tokio::test]
    async fn metrics_totals_returns_zeros_for_empty_table() {
        let db = setup_metrics_test_db().await.unwrap();

        let result = metrics_totals(&db.conn, None, 0).await.unwrap();
        assert_eq!(result.total_count, 0);
        assert_eq!(result.blocked_count, 0);
        assert_eq!(result.cached_count, 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.sum_duration, 0);
    }

    #[tokio::test]
    async fn top_clients_returns_ordered_by_count() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(1000, "low", 5, 0, 0, 0, 10),
            make_client_metrics(1000, "high", 20, 0, 0, 0, 10),
            make_client_metrics(1000, "mid", 10, 0, 0, 0, 10),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = top_clients(&db.conn, 0, 10).await.unwrap();
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
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = top_clients(&db.conn, 0, 10).await.unwrap();
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
        batch_upsert(&db.conn, rows).await.unwrap();

        let result = top_clients(&db.conn, 1500, 10).await.unwrap();
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
        batch_upsert(&db.conn, rows).await.unwrap();

        // width 1 keeps each bucket_ts separate, so this only checks clients are summed
        let result = timeline(&db.conn, 0, 1).await.unwrap();
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

    #[tokio::test]
    async fn timeline_rolls_up_by_width() {
        let db = setup_metrics_test_db().await.unwrap();
        let rows = vec![
            make_client_metrics(0, "a", 10, 0, 0, 0, 0),
            make_client_metrics(60_000, "a", 5, 0, 0, 0, 0),
            make_client_metrics(350_000, "a", 2, 0, 0, 0, 0),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        // with a 5 min width the first two fall in slot 0, the third in slot 300_000
        let result = timeline(&db.conn, 0, 5 * 60_000).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].ts, 0);
        assert_eq!(result[0].total, 15);
        assert_eq!(result[0].bucket_duration, 5 * 60_000);
        assert_eq!(result[1].ts, 300_000);
        assert_eq!(result[1].total, 2);
    }

    use crate::metrics::task::{DAY_MS, HOUR_MS, MINUTE_MS};

    #[tokio::test]
    async fn compress_before_rolls_up_minute_buckets() {
        let db = setup_metrics_test_db().await.unwrap();

        let rows = vec![
            make_client_metrics(HOUR_MS + MINUTE_MS, "a", 10, 2, 3, 1, 100),
            make_client_metrics(HOUR_MS + 2 * MINUTE_MS, "a", 5, 1, 1, 0, 50),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        compress_before(&db.conn, HOUR_MS * 3, HOUR_MS).await.unwrap();

        let result = list_range_client_metrics(&db.conn, 0, HOUR_MS * 5).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bucket_ts, HOUR_MS);
        assert_eq!(result[0].total_count, 15);
    }

    #[tokio::test]
    async fn compress_before_is_idempotent() {
        let db = setup_metrics_test_db().await.unwrap();

        let rows = vec![make_client_metrics(HOUR_MS + MINUTE_MS, "a", 10, 2, 3, 1, 100)];
        batch_upsert(&db.conn, rows).await.unwrap();

        compress_before(&db.conn, HOUR_MS * 3, HOUR_MS).await.unwrap();
        compress_before(&db.conn, HOUR_MS * 3, HOUR_MS).await.unwrap();

        let result = list_range_client_metrics(&db.conn, 0, HOUR_MS * 5).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].total_count, 10);
    }

    #[tokio::test]
    async fn compress_before_rolls_up_hour_buckets_into_day() {
        let db = setup_metrics_test_db().await.unwrap();

        let rows = vec![
            make_client_metrics(DAY_MS + HOUR_MS, "a", 10, 2, 3, 1, 100),
            make_client_metrics(DAY_MS + 2 * HOUR_MS, "a", 5, 1, 1, 0, 50),
        ];
        batch_upsert(&db.conn, rows).await.unwrap();

        compress_before(&db.conn, DAY_MS * 3, DAY_MS).await.unwrap();

        let result = list_range_client_metrics(&db.conn, 0, DAY_MS * 5).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bucket_ts, DAY_MS);
        assert_eq!(result[0].total_count, 15);
    }
}
