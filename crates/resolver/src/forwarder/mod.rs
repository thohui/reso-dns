use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;

use async_trait::async_trait;

use bytes::Bytes;
use reso_cache::{CacheKey, DnsResponseBytes};
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::helpers;
use reso_inflight::Inflight;
use tokio::time::Instant;
use upstream::{Limits, TcpPool, UdpPool, Upstreams};

use crate::DnsResolver;

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
                        max_total: 32,
                        max_idle: 1,
                        tcp_ttl: Duration::from_secs(30),
                        udp_sockets: 8,
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
            CacheKey::from_message(qmsg).context("failed to create cache key from message")?;

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

        let custom = resp_arc.as_ref().clone().into_custom_response(qmsg.id);
        Ok(custom)
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
                    match self.handle_tcp(raw, &upstream.tcp, deadline).await {
                        Ok(resp) => return Ok(resp),
                        Err(e) => {
                            tracing::warn!(upstream = %upstream.addr, error = %e, "TCP forward failed");
                            continue;
                        }
                    }
                }
                RequestType::UDP => {
                    // use a seeded slot so UDP & TCP progress similarly across upstreams
                    match self.handle_udp(raw, &upstream.udp, deadline).await {
                        Ok(resp) => {
                            match helpers::is_truncated(&resp) {
                                Some(true) => {
                                    // switch over to tcp if the response is truncated
                                    request_type = RequestType::TCP;
                                    match self.handle_tcp(raw, &upstream.tcp, deadline).await {
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
    async fn handle_udp(
        &self,
        query: &[u8],
        pool: &UdpPool,
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        let socket = pool.pick();
        socket.send_and_receive(query, deadline).await
    }
}
