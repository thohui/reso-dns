use crate::{
    database::models::client::{self, Client},
    global::SharedGlobal,
};
use axum::{Json, Router, extract::State, middleware, routing::get};

use super::{
    auth::{AllowedAuthMethods, auth_middleware},
    error::ApiError,
    extract::ApiQuery,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_clients_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
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

    let clients = client::list(&global.core_database, db_top, db_skip)
        .await
        .map_err(|e| {
            tracing::error!("failed to list clients: {:?}", e);
            ApiError::server_error()
        })?;

    let count: u64 = client::row_count(&global.core_database)
        .await
        .map_err(|e| {
            tracing::error!("failed to get client count: {:?}", e);
            ApiError::server_error()
        })?
        .try_into()
        .map_err(|_| ApiError::server_error())?;

    Ok(Json(PagedResponse::new(clients, Some(count), top, skip)))
}
