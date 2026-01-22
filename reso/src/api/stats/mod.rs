use axum::{Json, Router, extract::State, middleware, routing::get};

use crate::{global::SharedGlobal, metrics::service::LiveStats};

use super::auth::middleware::auth_middleware;

pub fn create_stats_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/live", get(live_stats))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn live_stats(global: State<SharedGlobal>) -> Json<LiveStats> {
    let stats = global.stats.live().await;
    Json(stats)
}
