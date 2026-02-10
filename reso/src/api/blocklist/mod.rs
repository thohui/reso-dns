use crate::{database::models::blocklist::BlockedDomain, global::SharedGlobal};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
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
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn list(
    query: Query<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<BlockedDomain>>, ApiError> {
    let top = query.top();
    let skip = query.skip();
    let blocked_domains = match BlockedDomain::list(&global.database, query.top(), query.skip()).await {
        Ok(domains) => domains,
        Err(e) => {
            tracing::error!("failed list blocked domains: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    let count = match BlockedDomain::row_count(&global.database).await {
        Ok(count) => count,
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
