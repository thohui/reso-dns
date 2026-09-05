use crate::{
    api::activity::Activity,
    database::models::{
        ListAction, MatchType, activity_log,
        domain_rule::{self, DomainRule},
        list_subscription,
    },
    global::SharedGlobal,
    uuid::EntityId,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post, put},
};
use serde::{Deserialize, Serialize};

use super::{
    auth::{AllowedAuthMethods, auth_middleware},
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
        .route("/{id}/details", get(details))
        .layer(middleware::from_fn_with_state(
            (global, AllowedAuthMethods::Session | AllowedAuthMethods::ApiKey),
            auth_middleware,
        ))
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

    let rules = domain_rule::list(&global.core_database, db_top, db_skip, search.clone())
        .await
        .map_err(|e| {
            tracing::error!("failed to list domain rules: {:?}", e);
            ApiError::server_error()
        })?;

    let count: u64 = domain_rule::count(&global.core_database, search)
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
    #[serde(default = "default_match_type")]
    match_type: MatchType,
    #[serde(default = "default_action")]
    action: ListAction,
}

fn default_match_type() -> MatchType {
    MatchType::Domain
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
    global
        .domain_rules
        .add_domain(&payload.domain, payload.match_type, payload.action)
        .await?;
    Ok(StatusCode::CREATED)
}

pub async fn remove_domain(global: State<SharedGlobal>, Json(payload): Json<DomainPayload>) -> Result<(), ApiError> {
    global.domain_rules.remove_domain(&payload.domain).await?;
    Ok(())
}

pub async fn toggle_domain(global: State<SharedGlobal>, Json(payload): Json<DomainPayload>) -> Result<(), ApiError> {
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
    global
        .domain_rules
        .update_domain_action(&payload.domain, payload.action)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct DetailsResponse {
    rule: DomainRule,
    subscription_name: Option<String>,
    total_blocked: i64,
    total_allowed: i64,
    last_seen_at: Option<i64>,
    activities: Vec<Activity>,
}

const DETAILS_ACTIVITY_LIMIT: i64 = 5;

pub async fn details(
    global: State<SharedGlobal>,
    Path(id): Path<EntityId<DomainRule>>,
) -> Result<Json<DetailsResponse>, ApiError> {
    let rule = domain_rule::get_by_id(&global.core_database, id)
        .await
        .map_err(|e| {
            tracing::error!("failed to get domain rule: {:?}", e);
            ApiError::server_error()
        })?
        .ok_or_else(ApiError::not_found)?;

    let subscription_name = match rule.subscription_id {
        Some(id) => list_subscription::name_by_id(&global.core_database, id)
            .await
            .map_err(|e| {
                tracing::error!("failed to get subscription name: {:?}", e);
                ApiError::server_error()
            })?,
        None => None,
    };

    let conn = &global.metrics_database;

    let (stats, recent) = tokio::join!(
        activity_log::stats_by_domain_rule(conn, id),
        activity_log::recent_by_rule(conn, id, DETAILS_ACTIVITY_LIMIT)
    );

    let (stats, recent) = match (stats, recent) {
        (Ok(stats), Ok(recent)) => (stats, recent),
        (Err(e), _) | (_, Err(e)) => {
            tracing::error!("failed to get activity for domain rule: {:?}", e);
            return Err(ApiError::server_error());
        }
    };

    let activities: Vec<Activity> = recent
        .into_iter()
        .map(Activity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            tracing::error!("failed to convert activity: {:?}", e);
            ApiError::server_error()
        })?;

    Ok(Json(DetailsResponse {
        rule,
        subscription_name,
        total_blocked: stats.blocked,
        total_allowed: stats.allowed,
        last_seen_at: stats.last_seen_at,
        activities,
    }))
}
