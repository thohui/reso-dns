use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    middleware,
    routing::{get, put},
};

use crate::{
    global::SharedGlobal,
    services::{self, config::model::Config},
};

use super::{auth::middleware::auth_middleware, error::ApiError};

pub fn create_config_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(config))
        .route("/", put(update))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn config(global: State<SharedGlobal>) -> Json<Arc<services::config::model::Config>> {
    Json(global.config_service.get_config())
}

pub async fn update(global: State<SharedGlobal>, Json(config): Json<Config>) -> Result<Json<Arc<Config>>, ApiError> {
    if let Err(e) = global.config_service.update_config(config).await {
        tracing::error!("failed to update config: {}", e);
        return Err(ApiError::server_error());
    }
    Ok(Json(global.config_service.get_config()))
}
