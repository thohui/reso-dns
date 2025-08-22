use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;

use bytes::Bytes;
use reso_context::DnsRequestCtx;
use tokio::{net::UdpSocket, time::timeout};

use crate::DnsResolver;

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstream: SocketAddr,
}

impl ForwardResolver {
    pub async fn new(upstream: SocketAddr) -> anyhow::Result<Self> {
        Ok(Self { upstream })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for ForwardResolver
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve<'a>(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes> {
        // perf: can we prevent creating a socket on every request? should be fine for now.
        let sock = UdpSocket::bind("0.0.0.0:0").await?;
        sock.send_to(ctx.raw(), self.upstream).await?;

        let mut buf = vec![0u8; 1232];
        let (n, _) = timeout(Duration::from_secs(2), sock.recv_from(&mut buf)).await??;
        buf.truncate(n);

        let resp_bytes = Bytes::from(buf);

        Ok(resp_bytes)
    }
}
