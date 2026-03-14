use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use reso_resolver::forwarder::resolver::ForwardResolver;
use reso_server::{DnsServer, ServerMiddlewares, ServerState};
use tokio_stream::wrappers::WatchStream;

use crate::{
    global::{Global, SharedGlobal},
    local::Local,
    middleware::{
        blocklist::BlocklistMiddleware, cache::CacheMiddleware, local_records::LocalRecordsMiddleware,
        metrics::MetricsMiddleware, ratelimit::RateLimitMiddleware, reso::ResoLocalMiddleware,
    },
    ratelimit::RateLimitConfig,
    services::{
        self,
        config::model::{ActiveResolver, Config, Upstream},
    },
};

pub fn server_middlewares(config: &Config) -> ServerMiddlewares<Global, Local> {
    let mut middlewares: Vec<Arc<dyn reso_context::DnsMiddleware<Global, Local> + 'static>> = vec![
        Arc::new(MetricsMiddleware),
        Arc::new(ResoLocalMiddleware::new()),
        Arc::new(LocalRecordsMiddleware),
    ];

    if config.dns.rate_limit.enabled {
        let ratelimit_config = RateLimitConfig {
            window_duration: Duration::from_secs(config.dns.rate_limit.window_duration as u64),
            max_queries_per_window: config.dns.rate_limit.max_queries_per_window,
        };
        middlewares.push(Arc::new(RateLimitMiddleware::new(ratelimit_config)));
    }

    middlewares.push(Arc::new(BlocklistMiddleware));
    middlewares.push(Arc::new(CacheMiddleware));

    Arc::new(middlewares)
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
        middlewares: server_middlewares(config),
        resolver: Arc::new(resolver),
    })
}

/// Starts a background task that updates the server state based on configuration change events.
pub async fn update_server_state_on_config_changes(global: SharedGlobal, server: Arc<DnsServer<Global, Local>>) {
    let mut rx = global.config.subscribe();

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
    let config = global.config.get_config();
    let server_state = create_server_state(&global, &config).await?;
    Ok(Arc::new(DnsServer::new(server_state)))
}
