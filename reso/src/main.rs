use std::{env, net::SocketAddr, sync::Arc};

use blocklist::service::BlocklistService;
use config::{DEFAULT_CONFIG_PATH, ResolverConfig};
use local::Local;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use moka::future::FutureExt;
use reso_cache::MessageCache;
use reso_dns::DnsMessage;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod blocklist;
mod config;
mod global;
mod local;
mod middleware;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dns_config_path = env::var("RESO_DNS_CONFIG").unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    let config = config::decode_from_path(&dns_config_path)?;

    let (nb, _guard) = non_blocking(std::io::stdout());

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(nb)
                .with_target(false)
                .with_filter(LevelFilter::from(config.server.log_level)),
        )
        .init();

    let connection = reso_database::connect(&config.database.path).await?;

    // ideally, we should setup a migration system for this
    let init_file = concat!("init.sql");
    let sql = std::fs::read_to_string(init_file)
        .map_err(|e| anyhow::anyhow!("Failed to read init file: {}", e))?;
    connection.execute(sql.as_str(), ()).await?;

    let server_addr = format!("{}:{}", config.server.ip, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address format");

    let global = Arc::new(global::Global::new(
        MessageCache::new(),
        BlocklistService::new(connection),
    ));

    #[allow(irrefutable_let_patterns)]
    let upstreams = if let ResolverConfig::Forwarder { upstreams } = config.resolver {
        upstreams
    } else {
        return Err(anyhow::anyhow!("Unsupported resolver configuration"));
    };

    let resolver = reso_resolver::forwarder::ForwardResolver::new(upstreams).await?;
    let mut server =
        reso_server::DnsServer::<_, _, Local>::new(server_addr, resolver, global.clone());

    server.add_success_handler(Arc::new(|ctx, resp| {
        async move {
            if !ctx.local().cache_hit {
                tracing::debug!("Cache miss for message ID: {}", ctx.message()?.id);
                let message = ctx.message()?;
                let resp_msg = DnsMessage::decode(resp)?;
                let _ = ctx
                    .global()
                    .cache
                    .insert(message, resp.clone(), resp_msg)
                    .await;
            } else {
                tracing::debug!("Cache hit for message ID: {}", ctx.message()?.id);
            }
            Ok(())
        }
        .boxed()
    }));

    server.add_middleware(BlocklistMiddleware);
    server.add_middleware(CacheMiddleware);

    server.run().await?;
    Ok(())
}
