use blocklist::{model::BlockedDomain, service::BlocklistService};
use cache::service::CacheService;
use config::DEFAULT_CONFIG_PATH;
use database::{DatabaseOperations, connect};
use global::Global;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use resolver::forwarder::ForwardResolver;
use server::DnsServer;
use std::{env, net::SocketAddr, sync::Arc};
use tracing::Level;

mod blocklist;
mod cache;
mod config;
mod database;
mod dns;
mod global;
mod middleware;
mod resolver;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let dns_config_path = env::var("DNS_CONFIG").unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    let config = config::decode_from_path(&dns_config_path)?;

    let connection = database::connect(&config.database.path).await?;

    let server_addr = format!("{}:{}", config.server.ip, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address format");

    let resolver = ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53))).await?;

    let global = Arc::new(Global::new(
        CacheService::new(),
        BlocklistService::new(connection),
    ));

    let server = DnsServer::new(server_addr, resolver, global);
    server.add_middleware(BlocklistMiddleware);
    server.add_middleware(CacheMiddleware);

    server.run().await?;

    Ok(())
}
