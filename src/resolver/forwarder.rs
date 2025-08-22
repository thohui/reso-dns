use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;

use bytes::Bytes;
use tokio::{net::UdpSocket, time::timeout};

use crate::dns::message::DnsMessage;

use super::{DnsRequestCtx, DnsResolver};

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
impl DnsResolver for ForwardResolver {
    async fn resolve(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Bytes> {
        // perf: can we prevent creating a socket on every request? should be fine for now.
        let sock = UdpSocket::bind("0.0.0.0:0").await?;
        sock.send_to(ctx.raw(), self.upstream).await?;

        let mut buf = vec![0u8; 1232];
        let (n, _) = timeout(Duration::from_secs(2), sock.recv_from(&mut buf)).await??;
        buf.truncate(n);

        let resp_bytes = Bytes::from(buf);

        let resp_bytes_cache = resp_bytes.clone();
        let cache = ctx.global.cache.clone();
        let query_msg_owned = ctx.message()?.clone();
        let resp_bytes_for_cache = resp_bytes.clone();

        tokio::spawn(async move {
            let resp_msg = DnsMessage::decode(&resp_bytes_cache).unwrap();
            let _ = cache
                .insert(&query_msg_owned, resp_bytes_for_cache, resp_msg)
                .await;
        });

        Ok(resp_bytes)
    }
}
