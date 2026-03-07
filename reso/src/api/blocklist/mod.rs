use crate::{database::models::blocked_domain::BlockedDomain, global::SharedGlobal};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;

use super::{
    auth::middleware::auth_middleware,
    error::ApiError,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_blocklist_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", delete(remove_domain))
        .route("/", post(add_domain))
        .route("/toggle", patch(toggle_domain))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn list(
    query: Query<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<BlockedDomain>>, ApiError> {
    let top = query.top();
    let skip = query.skip();

    let db_top: i64 = top.try_into().map_err(|_| {
        tracing::error!("top out of range: {}", top);
        ApiError::bad_request()
    })?;
    let db_skip: i64 = skip.try_into().map_err(|_| {
        tracing::error!("skip out of range: {}", skip);
        ApiError::bad_request()
    })?;

    let blocked_domains = match BlockedDomain::list(&global.core_database, db_top, db_skip).await {
        Ok(domains) => domains,
        Err(e) => {
            tracing::error!("failed list blocked domains: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    let count: u64 = match BlockedDomain::row_count(&global.core_database).await {
        Ok(count) => count.try_into().map_err(|_| {
            tracing::error!("negative row count: {}", count);
            ApiError::server_error()
        })?,
        Err(e) => {
            tracing::error!("failed to get blocked domain row count: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    Ok(Json(PagedResponse::new(blocked_domains, count, top, skip)))
}

#[derive(Deserialize)]
pub(crate) struct DomainPayload {
    domain: String,
}

pub async fn remove_domain(global: State<SharedGlobal>, Json(payload): Json<DomainPayload>) -> Result<(), ApiError> {
    if let Err(e) = global.blocklist.remove_domain(&payload.domain).await {
        tracing::error!("failed to delete domain: {:?}", e);
        return Err(ApiError::server_error());
    }

    Ok(())
}

pub async fn add_domain(
    global: State<SharedGlobal>,
    Json(payload): Json<DomainPayload>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = global.blocklist.add_domain(&payload.domain).await {
        tracing::error!("failed to add domain: {:?}", e);
        return Err(ApiError::server_error());
    }

    Ok(StatusCode::CREATED)
}

pub async fn toggle_domain(global: State<SharedGlobal>, Json(payload): Json<DomainPayload>) -> Result<(), ApiError> {
    if let Err(e) = global.blocklist.toggle_domain(&payload.domain).await {
        tracing::error!("failed to toggle domain: {:?}", e);
        return Err(ApiError::server_error());
    }

    Ok(())
}
