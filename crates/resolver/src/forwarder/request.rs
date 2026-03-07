use std::{sync::Arc, time::Duration};

use super::{tcp::TcpPool, upstream::Upstreams};
use crate::{
    ResolveError,
    forwarder::upstream::{Upstream, UpstreamError},
};
use bytes::Bytes;
use reso_context::{RequestBudget, RequestType};
use reso_dns::helpers;
use tracing::Instrument;

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

        let (pools, start) = self
            .upstreams
            .pick()
            .ok_or(ResolveError::Other("no upstreams available".into()))?;

        let request_tid = helpers::extract_transaction_id(&self.query)
            .ok_or(ResolveError::InvalidRequest("failed to extract tid from query".into()))?;

        let n = pools.len();
        let req_type = self.request_type;

        // Try each upstream in round robin order once.
        for off in 0..n {
            if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                return Err(ResolveError::Timeout);
            }

            // pick the next upstream in round-robin order.
            let idx = (start + off) % n;
            let upstream = &pools[idx];
            let span =
                tracing::debug_span!("upstream_attempt", upstream = %upstream.addr, attempt = off + 1, total = n);

            let attempt_res = async {
                match req_type {
                    RequestType::TCP | RequestType::DOH => self.resolve_tcp(&upstream.tcp, &self.query).await,
                    RequestType::UDP => {
                        match self.resolve_udp(upstream, &self.query).await {
                            Ok(resp) => match helpers::is_truncated(&resp) {
                                Some(true) => {
                                    if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                                        return Err(UpstreamError::Timeout);
                                    }
                                    // TCP fallback for THIS upstream only.
                                    self.resolve_tcp(&upstream.tcp, &self.query).await
                                }
                                Some(false) => Ok(resp),
                                None => Err(UpstreamError::Other("invalid UDP response".into())),
                            },
                            Err(e) => Err(e),
                        }
                    }
                }
            }
            .instrument(span)
            .await;

            let resp = match attempt_res {
                Ok(r) => {
                    upstream.record_success();
                    r
                }
                Err(ref e) => {
                    if matches!(
                        e,
                        UpstreamError::SendTimeout
                            | UpstreamError::RecvTimeout
                            | UpstreamError::SendError(_)
                            | UpstreamError::RecvError(_)
                            | UpstreamError::RecvTaskStopped
                    ) {
                        upstream.record_failure();
                    }

                    if let UpstreamError::RecvTaskStopped = *e {
                        upstream.clone().trigger_udp_reconnect();
                    }

                    tracing::warn!(
                        upstream = %upstream.addr,
                        req_type = ?req_type,
                        attempt = off + 1,
                        total = n,
                        error = ?e,
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

        Err(ResolveError::Other("all upstreams failed".into()))
    }

    /// Check if the request budget has at least `min` remaining.
    fn has_budget(&self, min: Duration) -> bool {
        self.request_budget.remaining().is_some_and(|r| r >= min)
    }

    /// Resolve the upstream request over tcp.
    async fn resolve_tcp(&self, pool: &TcpPool, query: &[u8]) -> Result<Bytes, UpstreamError> {
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
    async fn resolve_udp(&self, upstream: &Upstream, query: &[u8]) -> Result<Bytes, UpstreamError> {
        let deadline = self.request_budget.deadline();
        let udp = upstream.udp.load();
        udp.send_and_receive(query, deadline).await
    }
}
