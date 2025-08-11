use blocklist::service::BlocklistService;
use cache::service::CacheService;
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use resolver::forwarder::ForwardResolver;
use server::DnsServer;
use services::Services;
use std::{net::SocketAddr, sync::Arc};
use tracing::Level;

mod blocklist;
mod cache;
mod dns;
mod middleware;
mod resolver;
mod server;
mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let services = Arc::new(Services::new(CacheService::new(), BlocklistService::new()));

    services.blocklist.add_domain("google.com").await?;

    let resolver = ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53))).await?;

    let server = DnsServer::new("0.0.0.0:5300".parse()?, resolver, services);
    server.add_middleware(BlocklistMiddleware);
    server.add_middleware(CacheMiddleware);

    server.run().await?;

    Ok(())
}
