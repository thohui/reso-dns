use crate::{
    database::models::{ListAction, domain_rule::DomainRule},
    global::SharedGlobal,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post, put},
};
use serde::Deserialize;

use super::{
    auth::middleware::auth_middleware,
    error::ApiError,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_domain_rules_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", post(add_domain))
        .route("/", delete(remove_domain))
        .route("/", put(update_domain))
        .route("/toggle", patch(toggle_domain))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn list(
    query: Query<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<DomainRule>>, ApiError> {
    let top = query.top();
    let skip = query.skip();

    let db_top: i64 = top.try_into().map_err(|_| ApiError::bad_request())?;
    let db_skip: i64 = skip.try_into().map_err(|_| ApiError::bad_request())?;

    let search = query.search.clone();

    let rules = DomainRule::list(&global.core_database, db_top, db_skip, search.clone())
        .await
        .map_err(|e| {
            tracing::error!("failed to list domain rules: {:?}", e);
            ApiError::server_error()
        })?;

    let count: u64 = DomainRule::row_count(&global.core_database, search)
        .await
        .map_err(|e| {
            tracing::error!("failed to get domain rule row count: {:?}", e);
            ApiError::server_error()
        })?
        .try_into()
        .map_err(|_| ApiError::server_error())?;

    Ok(Json(PagedResponse::new(rules, Some(count), top, skip)))
}

#[derive(Deserialize)]
pub struct AddDomainPayload {
    domain: String,
    #[serde(default = "default_action")]
    action: ListAction,
}

fn default_action() -> ListAction {
    ListAction::Block
}

#[derive(Deserialize)]
pub struct DomainPayload {
    domain: String,
}

pub async fn add_domain(
    global: State<SharedGlobal>,
    Json(payload): Json<AddDomainPayload>,
) -> Result<StatusCode, ApiError> {
    global.domain_rules.add_domain(&payload.domain, payload.action).await?;
    Ok(StatusCode::CREATED)
}

pub async fn remove_domain(
    global: State<SharedGlobal>,
    Json(payload): Json<DomainPayload>,
) -> Result<(), ApiError> {
    global.domain_rules.remove_domain(&payload.domain).await?;
    Ok(())
}

pub async fn toggle_domain(
    global: State<SharedGlobal>,
    Json(payload): Json<DomainPayload>,
) -> Result<(), ApiError> {
    global.domain_rules.toggle_domain(&payload.domain).await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct UpdateDomainPayload {
    domain: String,
    action: ListAction,
}

pub async fn update_domain(
    global: State<SharedGlobal>,
    Json(payload): Json<UpdateDomainPayload>,
) -> Result<(), ApiError> {
    global.domain_rules.update_domain_action(&payload.domain, payload.action).await?;
    Ok(())
}
