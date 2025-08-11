use cache::service::CacheService;
use middleware::{TestMiddleware, cache::CacheMiddleware};
use resolver::forwarder::ForwardResolver;
use server::DnsServer;
use std::{net::SocketAddr, sync::Arc};

mod blocklist;
mod cache;
mod dns;
mod middleware;
mod resolver;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let resolver = ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53))).await?;

    let cache_service = Arc::new(CacheService::new());

    let server = DnsServer::new("0.0.0.0:5300".parse()?, resolver, cache_service.clone());
    server.add_middleware(TestMiddleware);
    server.add_middleware(CacheMiddleware);

    server.run().await?;

    Ok(())
}
