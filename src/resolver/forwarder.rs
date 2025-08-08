use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;

use bytes::Bytes;
use tokio::{net::UdpSocket, time::timeout};

use super::DnsResolver;

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    socket: UdpSocket,
}

impl ForwardResolver {
    pub async fn new(upstream: SocketAddr) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(upstream).await?;
        Ok(Self { socket })
    }
}

#[async_trait]
impl DnsResolver for ForwardResolver {
    async fn resolve(&self, query: &[u8]) -> anyhow::Result<Bytes> {
        self.socket.send(query).await?;

        let mut buf = vec![0u8; 1232];

        let n = timeout(Duration::from_secs(2), self.socket.recv(&mut buf)).await??;
        buf.truncate(n);

        Ok(Bytes::from(buf))
    }
}
