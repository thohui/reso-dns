use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Query, State},
    middleware,
    response::Result,
    routing::get,
};
use serde::Serialize;

use crate::{database::models::activity_log::ActivityLog, global::SharedGlobal};

use super::{
    auth::middleware::auth_middleware,
    error::ApiError,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_activity_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(activity))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}
pub async fn activity(
    global: State<SharedGlobal>,
    pagination: Query<PagedQuery>,
) -> Result<Json<PagedResponse<Activity>>, ApiError> {
    let conn = &global.database;

    let top = pagination.top();
    let skip = pagination.skip();

    let activity_logs = match ActivityLog::list(conn, top, skip).await {
        Ok(activities) => activities,
        Err(e) => {
            tracing::error!("failed to get activity logs: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    let row_count = match ActivityLog::row_count(conn).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("failed to get activity logs: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    let activities: Vec<Activity> = match activity_logs
        .into_iter()
        .map(Activity::try_from)
        .collect::<std::result::Result<Vec<_>, _>>()
    {
        Ok(activities) => activities,
        Err(e) => {
            tracing::error!("failed to convert activity: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    Ok(Json(PagedResponse::new(activities, row_count, top, skip)))
}

#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub timestamp: i64,
    pub transport: u8,
    pub client: Option<String>,
    pub duration: u64,
    pub qname: Option<String>,
    pub qtype: Option<i64>,
    #[serde(flatten)]
    pub kind: ActivityKind,
}

impl TryFrom<ActivityLog> for Activity {
    type Error = anyhow::Error;

    fn try_from(r: ActivityLog) -> Result<Self, Self::Error> {
        let transport: u8 = r
            .transport
            .try_into()
            .map_err(|_| anyhow::anyhow!("transport out of range: {}", r.transport))?;

        let kind = match r.kind.as_str() {
            "query" => {
                let rcode = r.rcode.context("query row missing rcode")? as u16;
                let blocked = r.blocked.context("query row missing blocked")?;
                let cache_hit = r.cache_hit.context("query row missing cache_hit")?;

                ActivityKind::Query(ActivityQuery {
                    source_id: r.source_id,
                    rcode,
                    blocked,
                    cache_hit,
                })
            }
            "error" => {
                let error_type = r.error_type.context("error row missing error_type")?;
                let message = r.error_message.context("error row missing error_message")?;

                ActivityKind::Error(ActivityError {
                    source_id: r.source_id,
                    error_type,
                    message,
                })
            }
            other => anyhow::bail!("unknown activity kind: {}", other),
        };

        Ok(Activity {
            timestamp: r.ts_ms,
            transport,
            client: Some(r.client),
            kind,
            duration: r.dur_ms,
            qname: r.qname,
            qtype: r.qtype,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "d")]
pub enum ActivityKind {
    #[serde(rename = "query")]
    Query(ActivityQuery),
    #[serde(rename = "error")]
    Error(ActivityError),
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityQuery {
    pub source_id: i64,
    pub rcode: u16,
    pub blocked: bool,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityError {
    pub source_id: i64,
    pub error_type: i64,
    pub message: String,
}
