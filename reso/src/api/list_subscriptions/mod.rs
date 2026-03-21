use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{ListAction, list_subscription::ListSubscription},
    global::SharedGlobal,
    utils::uuid::EntityId,
};

use super::{auth::middleware::auth_middleware, error::ApiError};

pub fn create_list_subscriptions_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", post(add))
        .route("/", delete(remove))
        .route("/toggle", patch(toggle))
        .route("/toggle-sync", patch(toggle_sync))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

#[derive(Serialize)]
pub struct ListSubscriptionResponse {
    pub id: EntityId<ListSubscription>,
    pub name: String,
    pub url: String,
    pub list_type: ListAction,
    pub enabled: bool,
    pub created_at: i64,
    pub last_synced_at: Option<i64>,
    pub domain_count: i64,
    pub sync_enabled: bool,
}

impl From<ListSubscription> for ListSubscriptionResponse {
    fn from(s: ListSubscription) -> Self {
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            list_type: s.list_type,
            enabled: s.enabled,
            created_at: s.created_at,
            last_synced_at: s.last_synced_at,
            domain_count: s.domain_count,
            sync_enabled: s.sync_enabled,
        }
    }
}

pub async fn list(global: State<SharedGlobal>) -> Result<Json<Vec<ListSubscriptionResponse>>, ApiError> {
    let subs = global.domain_rules.list_subscriptions().await?;
    Ok(Json(subs.into_iter().map(ListSubscriptionResponse::from).collect()))
}

#[derive(Deserialize)]
pub struct AddPayload {
    name: String,
    url: String,
    #[serde(default = "default_list_type")]
    list_type: ListAction,
    #[serde(default = "default_sync_enabled")]
    sync_enabled: bool,
}

fn default_list_type() -> ListAction {
    ListAction::Block
}

fn default_sync_enabled() -> bool {
    true
}

pub async fn add(global: State<SharedGlobal>, Json(payload): Json<AddPayload>) -> Result<StatusCode, ApiError> {
    let mut sub = ListSubscription::new(payload.name, payload.url);
    sub.list_type = payload.list_type;
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
