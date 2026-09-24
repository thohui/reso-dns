use std::net::IpAddr;

use axum::{Json, Router, extract::State, middleware, routing::get};
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{
        client_metrics::{self, TimelineBucket},
        domain_metrics,
    },
    global::SharedGlobal,
    metrics::service::LiveStats,
};

use super::{
    auth::{AllowedAuthMethods, auth_middleware},
    error::ApiError,
    extract::{ApiQuery, empty_as_none},
};

pub fn create_stats_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(stats))
        .route("/live", get(live_stats))
        .route("/top", get(top))
        .route("/timeline", get(timeline))
        .layer(middleware::from_fn_with_state(
            (global, AllowedAuthMethods::Session | AllowedAuthMethods::ApiKey),
            auth_middleware,
        ))
}

fn default_stats_range() -> TopRange {
    TopRange::All
}

#[derive(Deserialize)]
pub struct StatsQuery {
    #[serde(default, deserialize_with = "empty_as_none")]
    client: Option<IpAddr>,
    #[serde(default = "default_stats_range")]
    range: TopRange,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_queries: u64,
    /// Total queries blocked
    pub total_blocked: u64,
    /// Total queries cached
    pub total_cached: u64,
    /// Total errors
    pub total_errors: u64,
    /// Average response time
    pub average_response_time: u64,
}

impl From<client_metrics::GenericMetrics> for StatsResponse {
    fn from(value: client_metrics::GenericMetrics) -> Self {
        Self {
            total_queries: value.total_count as u64,
            total_blocked: value.blocked_count as u64,
            total_cached: value.cached_count as u64,
            total_errors: value.error_count as u64,
            average_response_time: value.sum_duration.checked_div(value.total_count).unwrap_or(0).max(0) as u64,
        }
    }
}

impl From<LiveStats> for StatsResponse {
    fn from(value: LiveStats) -> Self {
        let average = value.sum_duration.checked_div(value.total as u128).unwrap_or(0);
        Self {
            total_queries: value.total as u64,
            total_blocked: value.blocked as u64,
            total_cached: value.cached as u64,
            total_errors: value.errors as u64,
            average_response_time: u64::try_from(average).unwrap_or(u64::MAX),
        }
    }
}

pub async fn stats(global: State<SharedGlobal>, query: ApiQuery<StatsQuery>) -> Result<Json<StatsResponse>, ApiError> {
    if query.client.is_none() && matches!(query.range, TopRange::All) {
        return Ok(Json(StatsResponse::from(global.stats.live().await)));
    }
    let client = query.client.map(|c| c.to_canonical().to_string());
    let since = range_to_duration(&query.range);
    let metrics = client_metrics::metrics_totals(&global.metrics_database, client, since)
        .await
        .map_err(|e| {
            tracing::error!("failed to get metrics totals: {}", e);
            ApiError::server_error()
        })?;

    Ok(Json(StatsResponse::from(metrics)))
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

const MAX_TOP_LIMIT: usize = 100;

pub async fn top(global: State<SharedGlobal>, query: ApiQuery<TopQuery>) -> Result<Json<TopResponse>, ApiError> {
    let since = range_to_duration(&query.range);
    let db = &global.metrics_database;

    let db_top: i64 = query.top.try_into().map_err(|_| ApiError::bad_request())?;

    // Limit the maximum number of entries to prevent abuse
    if db_top <= 0 || db_top > MAX_TOP_LIMIT as i64 {
        return Err(ApiError::bad_request());
    }

    let (clients, domains, blocked_domains) = match tokio::join!(
        client_metrics::top_clients(db, since, db_top),
        domain_metrics::top_domains(db, since, db_top),
        domain_metrics::top_blocked(db, since, db_top)
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
    query: ApiQuery<TimelineQuery>,
) -> Result<Json<TimelineResponse>, ApiError> {
    let since = range_to_duration(&query.range);
    let bucket_width = range_to_bucket_width(&query.range);

    let buckets = client_metrics::timeline(&global.metrics_database, since, bucket_width)
        .await
        .map_err(|e| {
            tracing::error!("failed to get timeline: {}", e);
            ApiError::server_error()
        })?;

    Ok(Json(TimelineResponse { buckets }))
}

fn range_to_duration(range: &TopRange) -> i64 {
    let now = crate::time::now_millis();
    match range {
        TopRange::FiveMinutes => now - 5 * 60 * 1000,
        TopRange::Hour => now - 60 * 60 * 1000,
        TopRange::Day => now - 24 * 60 * 60 * 1000,
        TopRange::Week => now - 7 * 24 * 60 * 60 * 1000,
        TopRange::Month => now - 30 * 24 * 60 * 60 * 1000,
        TopRange::Year => now - 365 * 24 * 60 * 60 * 1000,
        TopRange::All => 0,
    }
}

// bucket width per range, at least as wide as the widest stored bucket so we only merge, never split
fn range_to_bucket_width(range: &TopRange) -> i64 {
    use crate::metrics::task::{DAY_MS, HOUR_MS, MINUTE_MS};
    match range {
        TopRange::FiveMinutes | TopRange::Hour => MINUTE_MS,
        TopRange::Day => 5 * MINUTE_MS,
        TopRange::Week => HOUR_MS,
        TopRange::Month => 3 * HOUR_MS,
        TopRange::Year | TopRange::All => DAY_MS,
    }
}
