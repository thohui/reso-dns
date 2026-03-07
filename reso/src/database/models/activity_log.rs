use anyhow::Context;
use rusqlite::types::Value;

use crate::database::MetricsDatabasePool;
use crate::database::models::Page;

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

#[derive(Debug, Default)]
pub struct ListFilter {
    pub client: Option<String>,
    pub qname: Option<String>,
    pub qtype: Option<i64>,
    pub blocked: Option<bool>,
    pub cache_hit: Option<bool>,
    pub rate_limited: Option<bool>,
    pub error_only: bool,
}

impl ListFilter {
    // param_offset: positional params already bound before filter params
    // (2 for list where ?1=limit ?2=offset, 0 for row_count)
    fn build_where(&self, param_offset: usize) -> (String, Vec<Value>) {
        let mut clauses: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();

        let mut push = |col: &str, val: Value| {
            params.push(val);
            clauses.push(format!("AND {col} = ?{}", params.len() + param_offset));
        };

        if let Some(ref v) = self.client {
            push("client", Value::Text(v.clone()));
        }
        if let Some(ref v) = self.qname {
            push("qname", Value::Text(v.clone()));
        }
        if let Some(v) = self.qtype {
            push("qtype", Value::Integer(v));
        }
        if let Some(v) = self.blocked {
            push("blocked", Value::Integer(v as i64));
        }
        if let Some(v) = self.cache_hit {
            push("cache_hit", Value::Integer(v as i64));
        }
        if let Some(v) = self.rate_limited {
            push("rate_limited", Value::Integer(v as i64));
        }
        if self.error_only {
            clauses.push("AND kind = 'error'".to_string());
        }

        (clauses.join(" "), params)
    }
}

pub enum SortColumn {
    Timestamp,
    Client,
    Qname,
    Duration,
}

impl SortColumn {
    fn as_sql(&self) -> &'static str {
        match self {
            SortColumn::Timestamp => "ts_ms",
            SortColumn::Client => "client",
            SortColumn::Qname => "qname",
            SortColumn::Duration => "dur_ms",
        }
    }
}

pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    fn as_sql(&self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> Result<ActivityLog, rusqlite::Error> {
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
}

impl ActivityLog {
    pub async fn list(
        db: &MetricsDatabasePool,
        limit: i64,
        offset: i64,
        filter: ListFilter,
        sort: SortColumn,
        dir: SortDir,
    ) -> anyhow::Result<Page<Self>> {
        Ok(db
            .interact(move |c| {
                let (where_clause, filter_params) = filter.build_where(2);

                let select_sql = format!(
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
                    WHERE 1=1 {where_clause}
                    ORDER BY {sort_col} {sort_dir}, source_id DESC
                    LIMIT ?1 OFFSET ?2
                    "#,
                    sort_col = sort.as_sql(),
                    sort_dir = dir.as_sql(),
                );

                let mut list_params: Vec<Value> = vec![Value::Integer(limit), Value::Integer(offset)];
                list_params.extend(filter_params);

                let items = {
                    let mut stmt = c.prepare(&select_sql)?;
                    let iter = stmt.query_map(rusqlite::params_from_iter(&list_params), map_row)?;
                    iter.collect::<Result<Vec<_>, rusqlite::Error>>()?
                };

                let (count_where, count_params) = filter.build_where(0);
                let count_sql = format!("SELECT COUNT(*) FROM activity_log WHERE 1=1 {count_where}");
                let total = c.query_row(&count_sql, rusqlite::params_from_iter(&count_params), |r| r.get(0))?;

                Ok(Page { items, total })
            })
            .await
            .context("failed to list activity logs")?)
    }
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

    async fn list_all(db: &MetricsDatabasePool, limit: i64, offset: i64) -> Page<ActivityLog> {
        ActivityLog::list(
            db,
            limit,
            offset,
            ListFilter::default(),
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_query_row_surfaces_as_query_kind() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();

        let page = list_all(&db.conn, 10, 0).await;
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);

        let r = &page.items[0];
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

        let page = list_all(&db.conn, 10, 0).await;
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);

        let r = &page.items[0];
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

        let page = list_all(&db.conn, 10, 0).await;
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].kind, "error");
        assert_eq!(page.items[1].kind, "query");
    }

    #[tokio::test]
    async fn test_page_total_includes_both_tables() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000), make_query(2000)])
            .await
            .unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(3000)]).await.unwrap();

        let page = list_all(&db.conn, 10, 0).await;
        assert_eq!(page.total, 3);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let db = setup_metrics_test_db().await.unwrap();
        let queries: Vec<_> = (1..=4).map(|i| make_query(i * 1000)).collect();
        DnsQueryLog::batch_insert(&db.conn, &queries).await.unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(5000)]).await.unwrap();

        let page1 = list_all(&db.conn, 3, 0).await;
        let page2 = list_all(&db.conn, 3, 3).await;

        assert_eq!(page1.items.len(), 3);
        assert_eq!(page1.total, 5);
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.total, 5);
    }

    #[tokio::test]
    async fn test_filter_error_only() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();
        DnsErrorLog::batch_insert(&db.conn, &[make_error(2000)]).await.unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                error_only: true,
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].kind, "error");
    }

    #[tokio::test]
    async fn test_filter_blocked() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut blocked = make_query(1000);
        blocked.blocked = true;
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), blocked])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                blocked: Some(true),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].blocked, Some(true));
    }

    #[tokio::test]
    async fn test_filter_qtype() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut quad_a = make_query(1000);
        quad_a.qtype = 28; // AAAA
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), quad_a])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                qtype: Some(28),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].qtype, Some(28));
    }

    #[tokio::test]
    async fn test_filter_cache_hit() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut cached = make_query(1000);
        cached.cache_hit = true;
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), cached])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                cache_hit: Some(true),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].cache_hit, Some(true));
    }

    #[tokio::test]
    async fn test_filter_rate_limited() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut limited = make_query(1000);
        limited.rate_limited = true;
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), limited])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                rate_limited: Some(true),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].rate_limited, Some(true));
    }

    #[tokio::test]
    async fn test_filter_client() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut other = make_query(1000);
        other.client = "10.0.0.1".to_string();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), other])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                client: Some("10.0.0.1".to_string()),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].client, "10.0.0.1");
    }

    #[tokio::test]
    async fn test_filter_qname() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut other = make_query(1000);
        other.qname = "other.com".to_string();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(2000), other])
            .await
            .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                qname: Some("other.com".to_string()),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].qname.as_deref(), Some("other.com"));
    }

    #[tokio::test]
    async fn test_filter_combined() {
        let db = setup_metrics_test_db().await.unwrap();
        let mut target = make_query(1000);
        target.client = "10.0.0.1".to_string();
        target.blocked = true;
        DnsQueryLog::batch_insert(
            &db.conn,
            &[
                make_query(2000),
                {
                    let mut q = make_query(3000);
                    q.client = "10.0.0.1".to_string();
                    q
                },
                target,
            ],
        )
        .await
        .unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                client: Some("10.0.0.1".to_string()),
                blocked: Some(true),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].client, "10.0.0.1");
        assert_eq!(page.items[0].blocked, Some(true));
    }

    #[tokio::test]
    async fn test_sort_by_duration_asc() {
        let db = setup_metrics_test_db().await.unwrap();
        let queries: Vec<_> = [30, 10, 20]
            .iter()
            .map(|&dur| {
                let mut q = make_query(1000);
                q.dur_ms = dur;
                q
            })
            .collect();
        DnsQueryLog::batch_insert(&db.conn, &queries).await.unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter::default(),
            SortColumn::Duration,
            SortDir::Asc,
        )
        .await
        .unwrap();

        let durations: Vec<i64> = page.items.iter().map(|r| r.dur_ms).collect();
        assert_eq!(durations, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_filter_client_sql_injection() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                client: Some("' OR '1'='1".to_string()),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 0, "injection payload should not match any rows");
        assert_eq!(page.total, 0, "injection payload should not match any rows");
    }

    #[tokio::test]
    async fn test_filter_qname_sql_injection() {
        let db = setup_metrics_test_db().await.unwrap();
        DnsQueryLog::batch_insert(&db.conn, &[make_query(1000)]).await.unwrap();

        let page = ActivityLog::list(
            &db.conn,
            10,
            0,
            ListFilter {
                qname: Some("example.com' OR '1'='1".to_string()),
                ..Default::default()
            },
            SortColumn::Timestamp,
            SortDir::Desc,
        )
        .await
        .unwrap();

        assert_eq!(page.items.len(), 0, "injection payload should not match any rows");
        assert_eq!(page.total, 0, "injection payload should not match any rows");
    }
}
