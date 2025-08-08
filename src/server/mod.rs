use std::{net::SocketAddr, sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{net::UdpSocket, time::timeout};

use crate::{middleware::DnsMiddleware, resolver::DnsResolver};

pub struct DnsServer<R> {
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    recv_size: usize,
    timeout: Duration,
    middlewares: Arc<Vec<Box<dyn DnsMiddleware>>>,
}

impl<R: DnsResolver + Send + Sync + 'static> DnsServer<R> {
    pub fn new(bind_addr: SocketAddr, resolver: R) -> Self {
        Self {
            bind_addr,
            resolver: Arc::new(resolver),
            recv_size: 1232, // edns safe
            timeout: Duration::from_secs(2),
            middlewares: Arc::new(Vec::new()),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.bind_addr).await?);

        loop {
            let mut query = vec![0u8; self.recv_size];
            let sock = socket.clone();
            let (len, client) = sock.recv_from(&mut query).await?;
            let resolver = self.resolver.clone();

            let duration = self.timeout;

            let middlewares = self.middlewares.clone();

            tokio::spawn(async move {
                if let Ok(Some(resp)) = run_middlewares(middlewares, &query).await {
                    let _ = sock.send_to(&resp, client).await;
                    return;
                }

                match timeout(duration, resolver.resolve(&query[0..len])).await {
                    Ok(Ok(resp)) => {
                        let _ = sock.send_to(&resp, client).await;
                    }
                    Ok(Err(e)) => {
                        println!("error: {}", e);
                    }
                    Err(e) => println!("timeout error: {}", e),
                }
            });
        }
    }
}

async fn run_middlewares(
    middlewares: Arc<Vec<Box<dyn DnsMiddleware>>>,
    packet: &[u8],
) -> anyhow::Result<Option<Bytes>> {
    for middleware in middlewares.iter() {
        if let Some(resp) = middleware.on_query(packet).await? {
            return Ok(Some(resp));
        }
    }
    Ok(None)
}
