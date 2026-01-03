use std::{net::SocketAddr, sync::Arc};

use auth::create_auth_router;
use axum::{Router, serve::Serve};
use stats::create_stats_router;
mod auth;
mod cookie;
mod error;
mod stats;

use crate::global::SharedGlobal;

pub async fn serve_web(global: SharedGlobal) -> anyhow::Result<()> {
    let addr = format!("{}:{}", global.config.server.http_ip, global.config.server.http_port)
        .parse::<SocketAddr>()
        .expect("invalid http server address format");

    let api = Router::new()
        .nest("/auth", create_auth_router())
        .nest("/stats", create_stats_router(global.clone()));

    let app = Router::new().nest("/api", api).with_state(global);

    tracing::info!("HTTP listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
