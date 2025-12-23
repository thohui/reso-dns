use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use blocklist::service::BlocklistService;
use bytes::Bytes;
use config::{DEFAULT_CONFIG_PATH, ResolverConfig, load_config};
use global::Global;
use local::Local;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use migrations::MIGRATIONS;
use moka::future::FutureExt;
use reso_cache::DnsMessageCache;
use reso_context::DnsRequestCtx;
use reso_dns::{DnsMessage, helpers};
use reso_resolver::{ResolveError, forwarder::resolver::ForwardResolver};
use reso_server::{DnsServer, ErrorCallback, ServerMiddlewares, ServerState, SuccessCallback};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod blocklist;
mod config;
mod global;
mod local;
mod middleware;
mod migrations;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (nb, _guard) = non_blocking(std::io::stdout());

    let dns_config_path = env::var("RESO_DNS_CONFIG").unwrap_or(DEFAULT_CONFIG_PATH.to_string());

    let config = load_config(&dns_config_path)?;

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(nb)
                .with_target(false)
                .with_filter(LevelFilter::from(config.server.log_level)),
        )
        .init();

    let connection = reso_database::connect(&config.database.path).await?;
    reso_database::run_migrations(&connection, MIGRATIONS).await?;
    let global = Arc::new(Global::new(
        DnsMessageCache::new(50_000),
        BlocklistService::new(connection),
    ));

    #[allow(irrefutable_let_patterns)]
    let upstreams = if let ResolverConfig::Forwarder { upstreams } = config.resolver {
        upstreams
    } else {
        return Err(anyhow::anyhow!("Unsupported resolver configuration"));
    };

    let resolver = ForwardResolver::new(&upstreams).await?;

    let timeout_duration = Duration::from_secs(config.server.timeout);

    let error_handler: ErrorCallback<Global, Local> =
        Arc::new(|ctx: &DnsRequestCtx<Global, Local>, err: &ResolveError| {
            async move {
                let id = helpers::extract_transaction_id(&ctx.raw()).unwrap_or_default();
                tracing::error!("error processing request: {}, error: {}", id, err,);
                Ok(())
            }
            .boxed()
        });

    let success_handler: SuccessCallback<Global, Local> =
        Arc::new(|ctx: &DnsRequestCtx<Global, Local>, resp: &Bytes| {
            async move {
                if !ctx.local().cache_hit {
                    let message = ctx.message()?;
                    let resp_msg = DnsMessage::decode(resp)?;
                    let _ = ctx.global().cache.insert(message, &resp_msg).await;
                }
                Ok(())
            }
            .boxed()
        });

    let middlewares: ServerMiddlewares<Global, Local> =
        Arc::new(vec![Arc::new(BlocklistMiddleware), Arc::new(CacheMiddleware)]);

    let state = ServerState {
        global: global.clone(),
        middlewares,
        on_error: Some(error_handler),
        on_success: Some(success_handler),
        resolver: Arc::new(resolver),
        timeout: timeout_duration,
    };

    let server = DnsServer::<_, Local>::new(state);

    global.blocklist.load_matcher().await?;

    let server_addr = format!("{}:{}", config.server.ip, config.server.port)
        .parse::<SocketAddr>()
        .expect("invalid server address format");

    tokio::select! {
        r = server.serve_tcp(server_addr) => {
            if let Err(e) = r {
                tracing::error!("TCP listener exited with error: {}", e);
            }
        },
        r = server.serve_udp(server_addr) => {
            if let Err(e) = r {
                tracing::error!("UDP listener exited with error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down DNS server...");
        },

    }

    Ok(())
}
