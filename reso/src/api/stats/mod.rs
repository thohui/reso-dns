use core::time;

use axum::{
    Json, Router,
    extract::{Query, State},
    middleware,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    database::models::client_metrics::{ClientMetrics, TimelineBucket},
    database::models::domain_metrics::DomainMetrics,
    global::SharedGlobal,
    metrics::service::LiveStats,
};

use super::{auth::middleware::auth_middleware, error::ApiError};

pub fn create_stats_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/live", get(live_stats))
        .route("/top", get(top))
        .route("/timeline", get(timeline))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn live_stats(global: State<SharedGlobal>) -> Json<LiveStats> {
    Json(global.stats.live().await)
}

fn default_top() -> usize {
    10
}

#[derive(Deserialize)]
pub struct TopQuery {
    #[serde(default = "default_top")]
    top: usize,
    #[serde(default = "default_range")]
    range: TopRange,
}

fn default_range() -> TopRange {
    TopRange::Day
}

#[derive(Deserialize)]
enum TopRange {
    #[serde(rename = "5min")]
    FiveMinutes,
    #[serde(rename = "hour")]
    Hour,
    #[serde(rename = "day")]
    Day,
    #[serde(rename = "week")]
    Week,
    #[serde(rename = "month")]
    Month,
    #[serde(rename = "year")]
    Year,
    #[serde(rename = "all")]
    All,
}

#[derive(Serialize)]
pub struct TopEntry {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct TopResponse {
    pub clients: Vec<TopEntry>,
    pub domains: Vec<TopEntry>,
    pub blocked_domains: Vec<TopEntry>,
}

pub async fn top(global: State<SharedGlobal>, query: Query<TopQuery>) -> Result<Json<TopResponse>, ApiError> {
    let since = range_to_duration(&query.range);
    let db = &global.metrics_database;

    let db_top: i64 = query.top.try_into().map_err(|_| ApiError::bad_request())?;

    let (clients, domains, blocked_domains) = match tokio::join!(
        ClientMetrics::top_clients(db, since, db_top),
        DomainMetrics::top_domains(db, since, db_top),
        DomainMetrics::top_blocked(db, since, db_top)
    ) {
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            tracing::error!("failed to get top stats: {}", e);
            return Err(ApiError::server_error());
        }
        (Ok(clients), Ok(domains), Ok(blocked_domains)) => (clients, domains, blocked_domains),
    };

    let to_entries = |rows: Vec<(String, i64)>| -> Vec<TopEntry> {
        rows.into_iter().map(|(name, count)| TopEntry { name, count }).collect()
    };

    Ok(Json(TopResponse {
        clients: to_entries(clients),
        domains: to_entries(domains),
        blocked_domains: to_entries(blocked_domains),
    }))
}

#[derive(Deserialize)]
pub struct TimelineQuery {
    #[serde(default = "default_range")]
    range: TopRange,
}

#[derive(Serialize)]
pub struct TimelineResponse {
    pub buckets: Vec<TimelineBucket>,
}

pub async fn timeline(
    global: State<SharedGlobal>,
    query: Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, ApiError> {
    let since = range_to_duration(&query.range);

    let buckets = ClientMetrics::timeline(&global.metrics_database, since)
        .await
        .map_err(|e| {
            tracing::error!("failed to get timeline: {}", e);
            ApiError::server_error()
        })?;

    Ok(Json(TimelineResponse { buckets }))
}

fn range_to_duration(range: &TopRange) -> i64 {
    let now = chrono::Utc::now();
    match range {
        TopRange::FiveMinutes => (now - chrono::Duration::minutes(5)).timestamp_millis(),
        TopRange::Hour => (now - chrono::Duration::hours(1)).timestamp_millis(),
        TopRange::Day => (now - chrono::Duration::days(1)).timestamp_millis(),
        TopRange::Week => (now - chrono::Duration::weeks(1)).timestamp_millis(),
        TopRange::Month => (now - chrono::Duration::days(30)).timestamp_millis(),
        TopRange::Year => (now - chrono::Duration::days(365)).timestamp_millis(),
        TopRange::All => 0,
    }
}
