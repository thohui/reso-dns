use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use async_trait::async_trait;

use bytes::Bytes;
use reso_context::DnsRequestCtx;
use tokio::{net::UdpSocket, time::timeout};

use crate::DnsResolver;

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstreams: Upstreams,
}

impl ForwardResolver {
    pub async fn new(upstreams: Vec<SocketAddr>) -> anyhow::Result<Self> {
        Ok(Self {
            upstreams: Upstreams::new(upstreams),
        })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for ForwardResolver
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve<'a>(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes> {
        let start = self.upstreams.pick_start();
        for upstream in self.upstreams.iter_from(start) {
            match self.resolve_with_upstream(upstream, ctx).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!("Failed to resolve with upstream {}: {}", upstream, e);
                }
            }
        }

        Err(anyhow::anyhow!(
            "All upstreams failed to resolve the request"
        ))
    }
}

impl ForwardResolver {
    async fn resolve_with_upstream<G, L>(
        &self,
        upstream: &SocketAddr,
        ctx: &DnsRequestCtx<G, L>,
    ) -> anyhow::Result<Bytes> {
        // perf: can we prevent creating a socket on every request? should be fine for now.

        let sock = UdpSocket::bind("0.0.0.0:0").await?;
        let mut buf = vec![0; 1232];

        sock.send_to(ctx.raw(), upstream).await?;

        let (n, _) = timeout(Duration::from_secs(2), sock.recv_from(&mut buf)).await??;

        buf.truncate(n);
        let resp_bytes = Bytes::from(buf);

        Ok(resp_bytes)
    }
}

/// A collection of upstream DNS servers to forward requests to.
struct Upstreams {
    upstreams: Arc<[SocketAddr]>,
    index: AtomicUsize,
}

impl Upstreams {
    fn new(upstreams: Vec<SocketAddr>) -> Self {
        Self {
            upstreams: upstreams.into(),
            index: AtomicUsize::new(0),
        }
    }

    fn pick_start(&self) -> usize {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        idx % self.upstreams.len()
    }

    fn iter_from(&self, start: usize) -> impl Iterator<Item = &SocketAddr> {
        self.upstreams
            .iter()
            .cycle()
            .skip(start)
            .take(self.upstreams.len())
    }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use super::Upstreams;

    #[test]
    pub fn test_upstream_pick() {
        let upstreams = Upstreams::new(vec![
            "0.0.0.0:0".parse::<SocketAddr>().unwrap(),
            "1.0.0.0:0".parse::<SocketAddr>().unwrap(),
            "2.0.0.0:0".parse::<SocketAddr>().unwrap(),
            "3.0.0.0:0".parse::<SocketAddr>().unwrap(),
        ]);

        let start_idx = upstreams.pick_start();
        let mut iter = upstreams.iter_from(start_idx);

        for i in 0..upstreams.upstreams.len() {
            let addr = iter.next().unwrap();
            assert_eq!(
                addr,
                &upstreams.upstreams[(start_idx + i) % upstreams.upstreams.len()]
            );
        }
    }
}
