use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::{database::models::list_subscription::ListSubscription, global::SharedGlobal, uuid::EntityId};

use super::{
    auth::{AllowedAuthMethods, auth_middleware},
    error::ApiError,
};

pub fn create_list_subscriptions_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", post(add))
        .route("/", delete(remove))
        .route("/toggle", patch(toggle))
        .route("/toggle-sync", patch(toggle_sync))
        .layer(middleware::from_fn_with_state(
            (global, AllowedAuthMethods::Session | AllowedAuthMethods::ApiKey),
            auth_middleware,
        ))
}

#[derive(Serialize)]
pub struct ListSubscriptionResponse {
    pub id: EntityId<ListSubscription>,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_synced_at: Option<i64>,
    pub domain_count: i64,
    pub sync_enabled: bool,
}

impl ListSubscriptionResponse {
    fn from_with_count(s: ListSubscription, domain_count: i64) -> Self {
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            enabled: s.enabled,
            created_at: s.created_at,
            last_synced_at: s.last_synced_at,
            domain_count,
            sync_enabled: s.sync_enabled,
        }
    }
}

pub async fn list(global: State<SharedGlobal>) -> Result<Json<Vec<ListSubscriptionResponse>>, ApiError> {
    let subs = global.domain_rules.list_subscriptions_with_counts().await?;
    Ok(Json(
        subs.into_iter()
            .map(|(s, count)| ListSubscriptionResponse::from_with_count(s, count))
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct AddPayload {
    name: String,
    url: String,
    #[serde(default = "default_sync_enabled")]
    sync_enabled: bool,
}

fn default_sync_enabled() -> bool {
    true
}

pub async fn add(global: State<SharedGlobal>, Json(payload): Json<AddPayload>) -> Result<StatusCode, ApiError> {
    let mut sub = ListSubscription::new(payload.name, payload.url);
    sub.sync_enabled = payload.sync_enabled;
    global.domain_rules.add_list_subscription(sub).await?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
pub struct IdPayload {
    id: EntityId<ListSubscription>,
}

pub async fn remove(global: State<SharedGlobal>, Json(payload): Json<IdPayload>) -> Result<(), ApiError> {
    global.domain_rules.remove_list_subscription(payload.id).await?;
    Ok(())
}

pub async fn toggle(global: State<SharedGlobal>, Json(payload): Json<IdPayload>) -> Result<(), ApiError> {
    global.domain_rules.toggle_list_subscription(payload.id).await?;
    Ok(())
}

pub async fn toggle_sync(global: State<SharedGlobal>, Json(payload): Json<IdPayload>) -> Result<(), ApiError> {
    global
        .domain_rules
        .toggle_list_subscription_sync_enabled(payload.id)
        .await?;
    Ok(())
}
