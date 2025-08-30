use std::{net::SocketAddr, time::Duration};

use anyhow::Context;

use async_trait::async_trait;

use bytes::Bytes;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::helpers;
use tokio::time::Instant;
use upstream::{Limits, TcpPool, UdpPool, Upstreams};

use crate::DnsResolver;

mod upstream;

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstreams: Upstreams,
}

impl ForwardResolver {
    pub async fn new(upstreams: &[SocketAddr]) -> anyhow::Result<Self> {
        if upstreams.is_empty() {
            tracing::warn!(
                "No upstreams configured for forward resolver, it will not be able to resolve any queries!"
            );
        }
        Ok(Self {
            upstreams: Upstreams::new(
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
        })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for ForwardResolver
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes> {
        let pools = &self.upstreams.as_slice();

        if pools.is_empty() {
            return Err(anyhow::anyhow!("no upstreams configured"));
        }

        let start = self.upstreams.pick_index().unwrap(); // safe: not empty
        let n = pools.len();

        let deadline = ctx.deadline(); // single global deadline
        let mut request_type = ctx.request_type();

        // try each upstream in round robin order
        for off in 0..n {
            let idx = (start + off) % n;
            let upstream = &pools[idx];

            match request_type {
                RequestType::TCP | RequestType::DOH => {
                    match self.handle_tcp(&upstream.tcp, ctx, deadline).await {
                        Ok(resp) => return Ok(resp),
                        Err(e) => {
                            tracing::warn!(upstream = %upstream.addr, error = %e, "TCP forward failed");
                            continue;
                        }
                    }
                }
                RequestType::UDP => {
                    // use a seeded slot so UDP & TCP progress similarly across upstreams
                    let seed = start + off;
                    match self.handle_udp(&upstream.udp, ctx, seed, deadline).await {
                        Ok(resp) => {
                            match helpers::is_truncated(&resp) {
                                Some(true) => {
                                    // switch over to tcp if the response is truncated
                                    request_type = RequestType::TCP;
                                    match self.handle_tcp(&upstream.tcp, ctx, deadline).await {
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
}

impl ForwardResolver {
    pub async fn handle_tcp<G, L>(
        &self,
        pool: &TcpPool,
        ctx: &DnsRequestCtx<G, L>,
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        let mut conn = pool.get_or_connect(deadline).await?;

        let resp_bytes = conn.send_and_receive(ctx.raw(), deadline).await?;

        pool.put_back(conn, true);

        Ok(resp_bytes)
    }

    /// Handle a UDP request by sending it to the specified address using the provided UDP pool.
    pub async fn handle_udp<G, L>(
        &self,
        pool: &UdpPool,
        ctx: &DnsRequestCtx<G, L>,
        seed: usize,
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        pool.pick_seeded(seed)
            .send_and_receive(ctx.raw(), deadline)
            .await
    }
}
