use resolver::forwarder::ForwardResolver;
use server::DnsServer;
use std::net::SocketAddr;

mod dns;
mod middleware;
mod resolver;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let resolver = ForwardResolver::new(SocketAddr::from(([1, 1, 1, 1], 53))).await?;

    let server = DnsServer::new("0.0.0.0:5300".parse()?, resolver);

    server.run().await?;

    Ok(())
}
