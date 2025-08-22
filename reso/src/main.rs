use std::{env, net::SocketAddr, sync::Arc};

use blocklist::service::BlocklistService;
use config::DEFAULT_CONFIG_PATH;
use local::Local;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use moka::future::FutureExt;
use reso_cache::{CacheKey, MessageCache};
use reso_dns::DnsMessage;
use tracing::Level;

mod blocklist;
mod config;
mod global;
mod local;
mod middleware;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let dns_config_path = env::var("DNS_CONFIG").unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    let config = config::decode_from_path(&dns_config_path)?;

    let connection = reso_database::connect(&config.database.path).await?;

    let server_addr = format!("{}:{}", config.server.ip, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address format");

    let global = Arc::new(global::Global::new(
        MessageCache::new(),
        BlocklistService::new(connection),
    ));

    let resolver =
        reso_resolver::forwarder::ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53)))
            .await?;
    let mut server =
        reso_server::DnsServer::<_, _, Local>::new(server_addr, resolver, global.clone());

    server.add_success_handler(Arc::new(|ctx, resp| {
        async move {
            if !ctx.local().cache_hit {
                tracing::debug!("Cache miss for message ID: {}", ctx.message()?.id);
                let message = ctx.message()?;
                let resp_msg = DnsMessage::decode(resp).unwrap();
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

    server.add_middleware(CacheMiddleware);
    server.add_middleware(BlocklistMiddleware);

    server.run().await?;
    Ok(())
}
