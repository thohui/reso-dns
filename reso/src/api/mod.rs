use std::net::SocketAddr;

use activity::create_activity_router;
use auth::create_auth_router;
use axum::{
    Router,
    body::Body,
    http::{
        HeaderValue, Response, StatusCode, Uri,
        header::{self, AUTHORIZATION, CONTENT_TYPE},
    },
    response::IntoResponse,
};
use blocklist::create_blocklist_router;
use mime_guess::from_path;
use stats::create_stats_router;
use tower_http::cors::{AllowMethods, CorsLayer};

mod activity;
mod auth;
mod blocklist;
mod cookie;
mod error;
mod pagination;
mod stats;

use crate::global::SharedGlobal;

pub async fn serve_web(global: SharedGlobal) -> anyhow::Result<()> {
    let addr = format!("{}:{}", global.config.server.http_ip, global.config.server.http_port)
        .parse::<SocketAddr>()
        .expect("invalid http server address format");

    let api = Router::new()
        .nest("/auth", create_auth_router(global.clone()))
        .nest("/stats", create_stats_router(global.clone()))
        .nest("/activity", create_activity_router(global.clone()))
        .nest("/blocklist", create_blocklist_router(global.clone()));

    let mut app = Router::new().nest("/api", api).with_state(global);

    #[cfg(feature = "embed-frontend")]
    {
        app = app.fallback(static_handler);
    }

    // Add support for vite dev server in debug mode.
    #[cfg(debug_assertions)]
    {
        let cors_layer = CorsLayer::new()
            .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
            .allow_credentials(true)
            .allow_methods(AllowMethods::mirror_request())
            .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
        app = app.layer(cors_layer);
    }

    tracing::info!("HTTP listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(feature = "embed-frontend")]
#[derive(rust_embed::RustEmbed)]
#[folder = "web/dist"]
pub struct Assets;

#[cfg(feature = "embed-frontend")]
async fn static_handler(uri: Uri) -> Response<Body> {
    let mut path = uri.path().trim_start_matches('/');

    let has_extension = path.rsplit_once('.').is_some();

    let is_index_html = !uri.path().contains("/assets") && !has_extension;

    if path.is_empty() || is_index_html {
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
