use std::{sync::Arc, time::Duration};

use futures::{FutureExt, StreamExt};
use reso_context::DnsRequestCtx;
use reso_dns::helpers;
use reso_resolver::forwarder::resolver::ForwardResolver;
use reso_server::{DnsServer, ErrorHandler, ServerError, ServerMiddlewares, ServerState};
use tokio_stream::wrappers::WatchStream;

use crate::{
    global::{Global, SharedGlobal},
    local::Local,
    metrics::event::ErrorLogEvent,
    middleware::{
        blocklist::BlocklistMiddleware, cache::CacheMiddleware, metrics::MetricsMiddleware,
        ratelimit::RateLimitMiddleware,
    },
    ratelimit::RateLimitConfig,
    services::{
        self,
        config::model::{ActiveResolver, Config, Upstream},
    },
};

pub fn error_handler() -> ErrorHandler<Global, Local> {
    Arc::new(|ctx: &DnsRequestCtx<Global, Local>, err: &ServerError| {
        async move {
            let local = ctx.local();
            let ts_ms: i64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("failed to get the system time")
                .as_millis() as i64;

            let mut qname = None;
            let mut qtype = None;

            // try to get qname and qtype
            if let Ok(msg) = ctx.message() {
                qname = msg.questions().first().and_then(|q| Some(q.qname.to_string()));
                qtype = msg.questions().first().and_then(|q| Some(q.qtype.to_u16() as i64));
            }

            let _ = ctx.global().metrics.error(ErrorLogEvent {
                ts_ms,
                client: ctx.request_address().to_string(),
                transport: ctx.request_type(),
                message: err.to_string(),
                r#type: err.error_type(),
                dur_ms: local.time_elapsed().as_millis() as u64,
                qname,
                qtype,
            });

            let id = helpers::extract_transaction_id(&ctx.raw()).unwrap_or(0);
            tracing::debug!("error processing request: {}: {:?}", id, err.to_string());
        }
        .boxed()
    })
}

pub fn server_middlewares(global: &SharedGlobal, config: &Config) -> ServerMiddlewares<Global, Local> {
    let ratelimit_config = RateLimitConfig {
        window_duration: Duration::from_secs(config.dns.rate_limit.window_duration as u64),
        max_queries_per_window: config.dns.rate_limit.max_queries_per_window,
    };

    let middlewares: ServerMiddlewares<Global, Local> = Arc::new(vec![
        Arc::new(MetricsMiddleware),
        Arc::new(RateLimitMiddleware::new(ratelimit_config)),
        Arc::new(BlocklistMiddleware),
        Arc::new(CacheMiddleware),
    ]);
    middlewares
}

/// Creates the new server state from a `services::config::model::Config`.
async fn create_server_state(
    global: &SharedGlobal,
    config: &services::config::model::Config,
) -> anyhow::Result<ServerState<Global, Local>> {
    let upstreams = config
        .dns
        .forwarder
        .upstreams()?
        .iter()
        .filter_map(|u| match u {
            // TODO: implement the rest.
            Upstream::Plain { endpoint } => endpoint.socket_addr().ok(),
            _ => None,
        })
        .collect::<Vec<_>>();

    let resolver = match &config.dns.active {
        ActiveResolver::Forwarder => ForwardResolver::new(&upstreams).await?,
    };

    Ok(ServerState {
        timeout: Duration::from_millis(config.dns.timeout),
        global: global.clone(),
        middlewares: server_middlewares(global, config),
        on_error: Some(error_handler()),
        resolver: Arc::new(resolver),
    })
}

/// Starts a background task that updates the server state based on configuration change events.
pub async fn update_server_state_on_config_changes(global: SharedGlobal, server: Arc<DnsServer<Global, Local>>) {
    let mut rx = global.config_service.subscribe();

    // skip the initial value.
    rx.mark_unchanged();

    let mut stream = WatchStream::from_changes(rx);
    while let Some(updated_config) = stream.next().await {
        tracing::info!("applying new configuration changes");
        match create_server_state(&global, &updated_config).await {
            Ok(state) => {
                server.swap_state(state);
            }
            Err(e) => {
                tracing::error!("failed to update dns server with new config: {}", e);
            }
        }
    }
}

pub async fn build_dns_server(global: SharedGlobal) -> anyhow::Result<Arc<DnsServer<Global, Local>>> {
    let config = global.config_service.get_config();
    let server_state = create_server_state(&global, &config).await?;
    Ok(Arc::new(DnsServer::new(server_state)))
}
