use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{api_key::ApiKey as DbApiKey, user::User},
    global::SharedGlobal,
    services::api_keys::{ApiKey, CreatedApiKey},
    utils::{now_millis, uuid::EntityId},
};

use super::{
    auth::{AllowedAuthMethods, middleware::auth_middleware},
    error::ApiError,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_api_keys_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", post(create))
        .route("/{id}", delete(remove))
        .layer(middleware::from_fn_with_state(
            (global, AllowedAuthMethods::Session),
            auth_middleware,
        ))
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub id: EntityId<DbApiKey>,
    pub display_name: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(k: ApiKey) -> Self {
        Self {
            id: k.id,
            display_name: k.display_name,
            created_by: k.created_by,
            created_at: k.created_at,
            expires_at: k.expires_at,
        }
    }
}

#[derive(Serialize)]
pub struct CreatedApiKeyResponse {
    pub id: EntityId<DbApiKey>,
    pub display_name: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub token: String,
}

impl From<CreatedApiKey> for CreatedApiKeyResponse {
    fn from(k: CreatedApiKey) -> Self {
        Self {
            id: k.id,
            display_name: k.display_name,
            created_by: k.created_by,
            created_at: k.created_at,
            expires_at: k.expires_at,
            token: k.token,
        }
    }
}

pub async fn list(
    query: Query<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<ApiKeyResponse>>, ApiError> {
    let top = query.top();
    let skip = query.skip();
    let search = query.search.clone();

    let db_top = top.try_into().map_err(|_| ApiError::bad_request())?;
    let db_skip = skip.try_into().map_err(|_| ApiError::bad_request())?;

    let page = global.api_keys.list_api_keys(db_top, db_skip, search).await.map_err(|e| {
        tracing::error!("failed to list api keys: {:?}", e);
        ApiError::server_error()
    })?;

    let total = page.total.map(|t| t as u64);
    let items: Vec<ApiKeyResponse> = page.items.into_iter().map(ApiKeyResponse::from).collect();

    Ok(Json(PagedResponse::new(items, total, top, skip)))
}

#[derive(Deserialize)]
pub struct CreatePayload {
    display_name: String,
    expires_at: Option<i64>,
}

pub async fn create(
    global: State<SharedGlobal>,
    Extension(user_id): Extension<EntityId<User>>,
    Json(payload): Json<CreatePayload>,
) -> Result<(StatusCode, Json<CreatedApiKeyResponse>), ApiError> {
    if let Some(expires_at) = payload.expires_at {
        if now_millis() > expires_at {
            return Err(ApiError::bad_request().with_message("API key cannot expire in the past"));
        }
    }

    let key = global
        .api_keys
        .create_api_key(payload.display_name, user_id, payload.expires_at)
        .await
        .map_err(|e| {
            tracing::error!("failed to create api key: {:?}", e);
            ApiError::server_error()
        })?;

    Ok((StatusCode::CREATED, Json(CreatedApiKeyResponse::from(key))))
}

#[derive(Deserialize)]
pub struct IdPath {
    id: EntityId<DbApiKey>,
}

pub async fn remove(
    global: State<SharedGlobal>,
    axum::extract::Path(path): axum::extract::Path<IdPath>,
) -> Result<(), ApiError> {
    global.api_keys.delete_api_key(&path.id).await.map_err(ApiError::from)?;
    Ok(())
}
