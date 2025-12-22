use std::{net::SocketAddr, sync::Arc, time::Duration};

use super::{tcp::TcpPool, udp::UdpConn, upstream::Upstreams};
use crate::ResolveError;
use bytes::Bytes;
use reso_context::{RequestBudget, RequestType};
use reso_dns::helpers;

pub struct UpstreamResolveRequest {
    request_type: RequestType,
    query: Bytes,
    request_budget: RequestBudget,
    upstreams: Arc<Upstreams>,
}

impl UpstreamResolveRequest {
    pub fn new(
        request_type: RequestType,
        query: Bytes,
        request_budget: RequestBudget,
        upstreams: Arc<Upstreams>,
    ) -> Self {
        Self {
            request_type,
            query,
            request_budget,
            upstreams,
        }
    }

    /// Resolve a DNS query by forwarding it to configured upstreams.
    pub async fn resolve(&self) -> Result<Bytes, ResolveError> {
        /// Minimum amount of time needed to start a new attempt.
        const MIN_REMAINING_TO_START_ATTEMPT: Duration = Duration::from_millis(15);

        let pools = self.upstreams.as_slice();
        if pools.is_empty() {
            return Err(ResolveError::Other(anyhow::anyhow!("no upstreams available")));
        }

        let start = self.upstreams.pick_index().unwrap(); // SAFE: we have alreay checked if the pool is not empty.

        let request_tid = helpers::extract_transaction_id(&self.query)
            .ok_or(ResolveError::InvalidRequest("failed to extract tid from query".into()))?;

        let n = pools.len();
        let req_type = self.request_type;

        // Try each upstream in round robin order once.
        for off in 0..n {
            // skip starting a new attempt if we're too close to deadline
            let remaining = match self.request_budget.remaining() {
                Some(r) => r,
                None => break,
            };

            if remaining < MIN_REMAINING_TO_START_ATTEMPT {
                return Err(ResolveError::Timeout);
            }

            let idx = (start + off) % n;
            let upstream = &pools[idx];

            let attempt_res = match req_type {
                RequestType::TCP | RequestType::DOH => self.resolve_tcp(&upstream.tcp_pool, &self.query).await,
                RequestType::UDP => {
                    match self.resolve_udp(upstream.addr, &self.query).await {
                        Ok(resp) => match helpers::is_truncated(&resp) {
                            Some(true) => {
                                // TCP fallback for THIS upstream only.
                                self.resolve_tcp(&upstream.tcp_pool, &self.query).await
                            }
                            Some(false) => Ok(resp),
                            None => Err(anyhow::anyhow!("invalid UDP response")),
                        },
                        Err(e) => Err(e),
                    }
                }
            };

            let resp = match attempt_res {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        upstream = %upstream.addr,
                        req_type = ?req_type,
                        error = %e,
                        "forward attempt failed"
                    );
                    continue;
                }
            };

            let response_tid = match helpers::extract_transaction_id(&resp) {
                Some(t) => t,
                None => {
                    tracing::warn!(
                        upstream = %upstream.addr,
                        req_type = ?req_type,
                        resp_len = resp.len(),
                        "response missing/invalid transaction id"
                    );
                    continue;
                }
            };

            if response_tid != request_tid {
                tracing::warn!(
                    upstream = %upstream.addr,
                    req_type = ?req_type,
                    expected_tid = request_tid,
                    got_tid = response_tid,
                    "transaction id mismatch"
                );
                continue;
            }
            return Ok(resp);
        }

        Err(ResolveError::Other(anyhow::anyhow!("all upstreams failed")))
    }

    /// Resolve the upstreqm request over tcp.
    async fn resolve_tcp(&self, pool: &TcpPool, query: &[u8]) -> anyhow::Result<Bytes> {
        let deadline = self.request_budget.deadline();
        let mut conn = pool.get_or_connect(deadline).await?;

        let result = conn.send_and_receive(query, deadline).await;

        match result {
            Ok(resp_bytes) => {
                pool.put_back(conn, true);
                Ok(resp_bytes)
            }
            Err(e) => {
                pool.put_back(conn, false);
                Err(e)
            }
        }
    }

    /// Resolve the upstream request over udp.
    async fn resolve_udp(&self, upstream_addr: SocketAddr, query: &[u8]) -> anyhow::Result<Bytes> {
        let deadline = self.request_budget.deadline();
        let connection = UdpConn::new(upstream_addr).await?;
        connection.send_and_receive(query, deadline).await
    }
}
