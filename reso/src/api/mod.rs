use std::{net::SocketAddr, sync::Arc};

use auth::create_auth_router;
use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Response, StatusCode, Uri, header},
    response::IntoResponse,
    routing::get,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;
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

    let app = Router::new()
        .nest("/api", api)
        .route("/", get(static_handler))
        .route("/{*path}", get(static_handler))
        .with_state(global);

    tracing::info!("HTTP listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

async fn static_handler(uri: Uri) -> Response<Body> {
    let mut path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = from_path(path).first_or_octet_stream();

            // cache files
            let cache = if path.contains('.') && !path.ends_with(".html") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
                .header(header::CACHE_CONTROL, cache)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
