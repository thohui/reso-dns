use std::{env, net::SocketAddr, rc::Rc, sync::Arc};

use blocklist::service::BlocklistService;
use config::{DEFAULT_CONFIG_PATH, ResolverConfig, load_config};
use global::Global;
use local::Local;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use migrations::MIGRATIONS;
use moka::future::FutureExt;
use reso_cache::DnsMessageCache;
use reso_dns::{DnsMessage, helpers};
use reso_resolver::forwarder::ForwardResolver;
use reso_server::DnsServer;
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

    let server_addr = format!("{}:{}", config.server.ip, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address format");

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

    let resolver = Arc::new(ForwardResolver::new(&upstreams).await?);
    let mut server = DnsServer::<_, _, Local>::new(server_addr, resolver, global.clone());

    server.add_success_handler(Arc::new(|ctx, resp| {
        async move {
            if !ctx.local().cache_hit {
                let message = ctx.message()?;
                let resp_msg = DnsMessage::decode(resp)?;
                let _ = ctx.global().cache.insert(message, &resp_msg).await;
            }
            Ok(())
        }
        .boxed()
    }));

    server.add_error_handler(Arc::new(|ctx, err| {
        async move {
            let id = helpers::extract_transaction_id(&ctx.raw()).unwrap_or_default();
            tracing::error!("Error processing request: {}, error: {}", id, err,);
            Ok(())
        }
        .boxed()
    }));

    server.add_middleware(BlocklistMiddleware);
    server.add_middleware(CacheMiddleware);

    global.blocklist.load_matcher().await?;

    tokio::select! {
        r = server.run(config.server.doh) => {
            if let Err(e) = r {
                tracing::error!("DNS server exited with error: {}", e);
            }
        },
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down DNS server...");
        },

    }

    Ok(())
}
