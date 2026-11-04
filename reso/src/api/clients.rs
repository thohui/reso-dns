use crate::{
    database::models::client::{self, Client},
    global::SharedGlobal,
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, put},
};
use serde::Deserialize;

use super::{
    auth::{AllowedAuthMethods, auth_middleware},
    error::ApiError,
    extract::{ApiJson, ApiPath, ApiQuery},
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_clients_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/{ip}", put(update))
        .layer(middleware::from_fn_with_state(
            (global, AllowedAuthMethods::Session | AllowedAuthMethods::ApiKey),
            auth_middleware,
        ))
}

pub async fn list(
    query: ApiQuery<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<Client>>, ApiError> {
    let top = query.top();
    let skip = query.skip();

    let db_top: i64 = top.try_into().map_err(|_| ApiError::bad_request())?;
    let db_skip: i64 = skip.try_into().map_err(|_| ApiError::bad_request())?;
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let clients = client::list(&global.core_database, db_top, db_skip, search.clone())
        .await
        .map_err(|e| {
            tracing::error!("failed to list clients: {:?}", e);
            ApiError::server_error()
        })?;

    let count: u64 = client::row_count(&global.core_database, search)
        .await
        .map_err(|e| {
            tracing::error!("failed to get client count: {:?}", e);
            ApiError::server_error()
        })?
        .try_into()
        .map_err(|_| ApiError::server_error())?;

    Ok(Json(PagedResponse::new(clients, Some(count), top, skip)))
}

const MAX_DISPLAY_NAME_LEN: usize = 255;

#[derive(Deserialize)]
pub struct UpdatePayload {
    /// empty string or `None` clears the name.
    #[serde(deserialize_with = "Option::deserialize")]
    display_name: Option<String>,
}

pub async fn update(
    global: State<SharedGlobal>,
    ApiPath(ip): ApiPath<String>,
    ApiJson(payload): ApiJson<UpdatePayload>,
) -> Result<StatusCode, ApiError> {
    let display_name = payload
        .display_name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    if display_name
        .as_ref()
        .is_some_and(|n| n.chars().count() > MAX_DISPLAY_NAME_LEN)
    {
        return Err(ApiError::bad_request().with_message("Display name is too long"));
    }

    let updated = client::set_display_name(&global.core_database, ip, display_name)
        .await
        .map_err(|e| {
            tracing::error!("failed to update client: {:?}", e);
            ApiError::server_error()
        })?;

    if !updated {
        return Err(ApiError::not_found());
    }

    Ok(StatusCode::NO_CONTENT)
}
