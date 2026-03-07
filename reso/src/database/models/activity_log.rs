use anyhow::Context;
use rusqlite::params;

use crate::database::MetricsDatabasePool;

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
    pub rate_limited: Option<bool>,
    pub dur_ms: i64,

    pub error_type: Option<i64>,
    pub error_message: Option<String>,
}

impl ActivityLog {
    pub async fn list(db: &MetricsDatabasePool, limit: i64, offset: i64) -> anyhow::Result<Vec<Self>> {
        Ok(db
            .interact(move |c| {
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
                      error_message,
                      rate_limited
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
                        rate_limited: row.get(13)?,
                    })
                })?;

                iter.collect::<Result<Vec<_>, rusqlite::Error>>()
            })
            .await
            .context("failed to list activity logs")?)
    }

    pub async fn row_count(db: &MetricsDatabasePool) -> anyhow::Result<i64> {
        Ok(db
            .interact(|c| c.query_row("SELECT COUNT(*) FROM activity_log", [], |r| r.get(0)))
            .await
            .context("failed to count activity logs")?)
    }
}

pub enum ListFilter {
    All,
    CacheHit,
    RateLimit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        models::{error_log::DnsErrorLog, query_log::DnsQueryLog},
        setup_metrics_test_db,
    };

    fn make_query(ts_ms: i64) -> DnsQueryLog {
        DnsQueryLog {
            ts_ms,
            transport: 1,
            client: "127.0.0.1".to_string(),
            qname: "example.com".to_string(),
            qtype: 1,
            rcode: 0,
            blocked: false,
            cache_hit: false,
            dur_ms: 10,
            rate_limited: false,
        }
    }

    fn make_error(ts_ms: i64) -> DnsErrorLog {
        DnsErrorLog {
            ts_ms,
            transport: 1,
            client: "127.0.0.1".to_string(),
            message: "timeout".to_string(),
            r#type: 1,
            dur_ms: 50,
            qname: Some("fail.com".to_string()),
            qtype: Some(1),
        }
    }

    #[tokio::test]
    async fn test_query_row_surfaces_as_query_kind() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();

        let results = ActivityLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.kind, "query");
        assert_eq!(r.ts_ms, 1000);
        assert_eq!(r.client, "127.0.0.1");
        assert_eq!(r.qname.as_deref(), Some("example.com"));
        assert!(r.rcode.is_some());
        assert!(r.blocked.is_some());
        assert!(r.cache_hit.is_some());
        assert!(r.error_type.is_none());
        assert!(r.error_message.is_none());
    }

    #[tokio::test]
    async fn test_error_row_surfaces_as_error_kind() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(1000)]).await.unwrap();

        let results = ActivityLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);

        let r = &results[0];
        assert_eq!(r.kind, "error");
        assert_eq!(r.ts_ms, 1000);
        assert_eq!(r.client, "127.0.0.1");
        assert!(r.error_type.is_some());
        assert!(r.error_message.is_some());
        assert!(r.rcode.is_none());
        assert!(r.blocked.is_none());
        assert!(r.cache_hit.is_none());
    }

    #[tokio::test]
    async fn test_errors_ordered_before_queries_at_same_timestamp() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(1000)]).await.unwrap();

        let results = ActivityLog::list(&db.conn, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].kind, "error");
        assert_eq!(results[1].kind, "query");
    }

    #[tokio::test]
    async fn test_row_count_includes_both_tables() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000), make_query(2000)])
            .await
            .unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(3000)]).await.unwrap();

        let count = ActivityLog::row_count(&db.conn).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let db = setup_metrics_test_db().await.unwrap();
        let queries: Vec<_> = (1..=4).map(|i| make_query(i * 1000)).collect();
        DnsQueryLog::batch_insert(&db.conn, &queries).await.unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(5000)]).await.unwrap();

        let page1 = ActivityLog::list(&db.conn, 3, 0).await.unwrap();
        let page2 = ActivityLog::list(&db.conn, 3, 3).await.unwrap();

        assert_eq!(page1.len(), 3);
        assert_eq!(page2.len(), 2);
    }
}
