use cache::service::CacheService;
use middleware::cache::CacheMiddleware;
use resolver::forwarder::ForwardResolver;
use server::DnsServer;
use std::{net::SocketAddr, sync::Arc};
use tracing::Level;

mod blocklist;
mod cache;
mod dns;
mod middleware;
mod resolver;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let resolver = ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53))).await?;

    let cache_service = Arc::new(CacheService::new());

    let server = DnsServer::new("0.0.0.0:5300".parse()?, resolver, cache_service.clone());
    server.add_middleware(CacheMiddleware);

    server.run().await?;

    Ok(())
}
