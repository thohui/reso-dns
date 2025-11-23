use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;

use async_trait::async_trait;

use bytes::{Bytes, BytesMut};
use reso_cache::CacheKey;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::helpers;
use reso_inflight::Inflight;
use tcp::TcpPool;
use tokio::time::Instant;
use udp::UdpConn;
use upstream::{Limits, Upstreams};

use crate::DnsResolver;

mod tcp;
mod udp;
mod upstream;

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstreams: Arc<Upstreams>,
    inflight: Inflight<CacheKey, DnsResponseBytes>,
}

impl ForwardResolver {
    pub async fn new(upstreams: &[SocketAddr]) -> anyhow::Result<Self> {
        if upstreams.is_empty() {
            tracing::warn!(
                "No upstreams configured for forward resolver, it will not be able to resolve any queries!"
            );
        }
        Ok(Self {
            inflight: Inflight::new(),
            upstreams: Arc::new(
                Upstreams::new(
                    upstreams,
                    // TODO: make this configurable
                    Limits {
                        connect_timeout: Duration::from_secs(5),
                        max_tcp_connections: 100,
                        max_idle_tcp_connections: 100,
                        tcp_ttl: Duration::from_secs(30),
                    },
                )
                .await?,
            ),
        })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for Arc<ForwardResolver>
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes> {
        let qmsg = ctx.message()?;
        let cache_key =
            CacheKey::try_from(qmsg).context("failed to create cache key from message")?;

        let raw = ctx.raw();
        let req_type = ctx.request_type();
        let deadline = ctx.deadline();
        let this = Arc::clone(self);

        let resp_arc = self
            .inflight
            .get_or_run(cache_key, async move |_| {
                let bytes = this.resolve_inner(&raw, req_type, deadline).await?;
                Ok(DnsResponseBytes::new(bytes))
            })
            .await?;

        let resp = resp_arc.as_ref().clone().into_custom_response(qmsg.id);
        Ok(resp)
    }
}

impl ForwardResolver {
    async fn resolve_inner(
        &self,
        raw: &[u8],
        mut request_type: RequestType,
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        let pools = &self.upstreams.as_slice();

        if pools.is_empty() {
            return Err(anyhow::anyhow!("no upstreams configured"));
        }

        let start = self.upstreams.pick_index().unwrap(); // safe: not empty
        let n = pools.len();

        // try each upstream in round robin order
        for off in 0..n {
            let idx = (start + off) % n;
            let upstream = &pools[idx];

            match request_type {
                RequestType::TCP | RequestType::DOH => {
                    match self.handle_tcp(raw, &upstream.tcp_pool, deadline).await {
                        Ok(resp) => return Ok(resp),
                        Err(e) => {
                            tracing::warn!(upstream = %upstream.addr, error = %e, "TCP forward failed");
                            continue;
                        }
                    }
                }
                RequestType::UDP => {
                    // use a seeded slot so UDP & TCP progress similarly across upstreams
                    match self.handle_udp(raw, deadline).await {
                        Ok(resp) => {
                            match helpers::is_truncated(&resp) {
                                Some(true) => {
                                    // switch over to tcp if the response is truncated
                                    request_type = RequestType::TCP;
                                    match self.handle_tcp(raw, &upstream.tcp_pool, deadline).await {
                                        Ok(tcp_resp) => return Ok(tcp_resp),
                                        Err(e) => {
                                            tracing::warn!(upstream = %upstream.addr, error = %e,
                                                "TCP fallback after truncation failed");
                                            continue;
                                        }
                                    }
                                }
                                Some(false) => return Ok(resp),
                                None => {
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(upstream = %upstream.addr, error = %e, "UDP forward failed");
                            continue;
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("all upstreams failed"))
    }

    async fn handle_tcp(
        &self,
        query: &[u8],
        pool: &TcpPool,
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        let mut conn = pool.get_or_connect(deadline).await?;
        let resp_bytes = conn.send_and_receive(query, deadline).await?;

        pool.put_back(conn, true);

        Ok(resp_bytes)
    }

    /// Handle a UDP request by sending it to the specified address using the provided UDP pool.
    async fn handle_udp(&self, query: &[u8], deadline: Instant) -> anyhow::Result<Bytes> {
        let remote_addr = self
            .upstreams
            .pick()
            .context("no upstreams available")?
            .addr;

        let connection = UdpConn::new(remote_addr).await?;
        connection.send_and_receive(query, deadline).await
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct DnsResponseBytes(Bytes);

impl DnsResponseBytes {
    pub fn new(bytes: Bytes) -> Self {
        Self(bytes)
    }

    pub fn into_custom_response(self, transaction_id: u16) -> Bytes {
        let mut bytes = BytesMut::from(&self.0[0..]);
        // overwrite the transaction id.
        bytes[0] = (transaction_id >> 8) as u8;
        bytes[1] = (transaction_id & 0xFF) as u8;
        bytes.freeze()
    }
}
