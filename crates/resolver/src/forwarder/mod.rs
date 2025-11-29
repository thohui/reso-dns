use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;

use async_trait::async_trait;

use bytes::{Bytes, BytesMut};
use rand::Rng;
use reso_cache::CacheKey;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, helpers};
use reso_inflight::Inflight;
use tcp::TcpPool;
use tokio::time::Instant;
use udp::UdpConn;
use upstream::{Limits, Upstreams};

use crate::{DnsResolver, ResolveError};

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
            inflight: Inflight::new(),
        })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for Arc<ForwardResolver>
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> Result<Bytes, ResolveError> {
        let qmsg = ctx.message().map_err(ResolveError::Decode)?;

        let cache_key = CacheKey::try_from(qmsg).map_err(ResolveError::Other)?;

        let raw = ctx.raw();
        let req_type = ctx.request_type();
        let deadline = ctx.deadline();
        let this = Arc::clone(self);

        let resp_arc = self
            .inflight
            .get_or_run(cache_key, async move |_| {
                // generate a new transaction ID for the upstream request.
                let (raw, tid) = this.generate_tid(&raw);
                let resp = this.resolve_inner(&raw, req_type, deadline).await?;

                // verify that the response transaction ID matches the request ID
                if helpers::extract_transaction_id(&resp) != Some(tid) {
                    return Err(anyhow::anyhow!(
                        "upstream response transaction ID does not match request ID"
                    ));
                }

                Ok(DnsResponseBytes::new(resp))
            })
            .await
            .map_err(ResolveError::Other)?;

        let resp = resp_arc
            .as_ref()
            .clone()
            .into_custom_response(helpers::extract_transaction_id(&ctx.raw()).unwrap_or_default());

        let resp_message = DnsMessage::decode(&resp).map_err(ResolveError::Decode)?;

        // ensure that both request and response have exactly one question
        if resp_message.questions().len() != 1 {
            return Err(ResolveError::InvalidResponse(std::format!(
                "upstream response contains {} questions, expected 1",
                resp_message.questions().len(),
            )));
        }

        let req_q = qmsg.questions().first();
        let resp_q = resp_message.questions().first();

        // ensure that the response question matches the request question
        if req_q != resp_q {
            return Err(ResolveError::InvalidResponse(
                "upstream response question does not match request question".to_string(),
            ));
        }

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

    /// Modify the transaction ID of the given query to a random value to prevent poisoning attacks.
    fn generate_tid(&self, query: &[u8]) -> (Bytes, u16) {
        let mut rng = rand::rng();

        let randomized_id = rng.random::<u16>();

        let mut bytes = BytesMut::from(&query[0..]);
        // overwrite the transaction id.
        bytes[0] = (randomized_id >> 8) as u8;
        bytes[1] = (randomized_id & 0xFF) as u8;

        (bytes.freeze(), randomized_id)
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
