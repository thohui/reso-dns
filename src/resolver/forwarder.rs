use std::net::SocketAddr;

use async_trait::async_trait;

use bytes::Bytes;
use tokio::net::UdpSocket;

use super::{DnsRequestCtx, DnsResolver};

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
    async fn resolve(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Bytes> {
        self.socket.send(ctx.raw).await?;

        let mut buf = vec![0u8; 1232];

        let n = self.socket.recv(&mut buf).await?;
        buf.truncate(n);

        Ok(Bytes::from(buf))
    }
}
