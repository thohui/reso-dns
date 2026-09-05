use std::{sync::Arc, time::Duration};

use super::{tcp::TcpPool, upstream::Upstreams};
use crate::{
    ResolveError, ResolveErrorKind,
    forwarder::upstream::{Upstream, UpstreamError},
};
use bytes::Bytes;
use reso_context::{DnsProtocol, RequestBudget};
use reso_dns::helpers;
use tracing::Instrument;

const MIN_REMAINING_TO_START_ATTEMPT: Duration = Duration::from_millis(15);

pub struct UpstreamResolveRequest {
    request_type: DnsProtocol,
    query: Bytes,
    request_budget: RequestBudget,
    upstreams: Arc<Upstreams>,
}

impl UpstreamResolveRequest {
    pub fn new(
        request_type: DnsProtocol,
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

    pub async fn resolve(&self) -> Result<(Bytes, DnsProtocol), ResolveError> {
        let upstreams = self
            .upstreams
            .iter()
            .ok_or(ResolveErrorKind::Other("no upstreams available".into()))?;

        let request_tid = helpers::extract_transaction_id(&self.query).ok_or(ResolveErrorKind::InvalidRequest(
            "failed to extract tid from query".into(),
        ))?;

        let req_type = self.request_type;

        let mut last_protocol: Option<DnsProtocol> = None;

        // Try each upstream in round robin order once.
        for (attempt, upstream) in upstreams.enumerate() {
            if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                return Err(ResolveErrorKind::Timeout.with_protocol(last_protocol));
            }

            let span = tracing::debug_span!("upstream_attempt", upstream = %upstream.addr, attempt=attempt);

            let (protocol, attempt_res) = self.try_upstream(&upstream, req_type).instrument(span).await;
            last_protocol = Some(protocol);

            let resp = match attempt_res {
                Ok(r) => {
                    upstream.health.record_success(upstream.addr);
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
                            | UpstreamError::Tls(_)
                    ) {
                        upstream.health.record_failure(upstream.addr);
                    }

                    if let UpstreamError::RecvTaskStopped = *e {
                        upstream.clone().trigger_udp_reconnect();
                    }

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
            return Ok((resp, protocol));
        }

        Err(ResolveErrorKind::Other("all upstreams failed".into()).with_protocol(last_protocol))
    }

    /// Attempt a single upstream.
    async fn try_upstream(
        &self,
        upstream: &Upstream,
        incoming_request_type: DnsProtocol,
    ) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        match incoming_request_type {
            DnsProtocol::TCP => self.try_tcp(&upstream.tcp).await,
            // A DoT upstream has no UDP mux, so it is forced to go over tcp (DoT).
            DnsProtocol::UDP if upstream.udp.is_none() => self.try_tcp(&upstream.tcp).await,
            DnsProtocol::UDP => self.resolve_udp_with_fallback(upstream).await,
            DnsProtocol::DOT | DnsProtocol::DOH => self.try_tcp(&upstream.tcp).await,
        }
    }

    /// Resolve over TCP or DoT
    async fn try_tcp(&self, pool: &TcpPool) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        let protocol = if pool.has_tls() {
            DnsProtocol::DOT
        } else {
            DnsProtocol::TCP
        };
        (protocol, self.resolve_tcp(pool, &self.query).await)
    }

    async fn resolve_udp_with_fallback(&self, upstream: &Upstream) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        let resp = match self.resolve_udp(upstream, &self.query).await {
            Ok(resp) => resp,
            Err(e) => return (DnsProtocol::UDP, Err(e)),
        };

        match helpers::is_truncated(&resp) {
            Some(true) => {
                if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                    return (DnsProtocol::UDP, Err(UpstreamError::Timeout));
                }
                // TCP fallback for THIS upstream only.
                self.try_tcp(&upstream.tcp).await
            }
            Some(false) => (DnsProtocol::UDP, Ok(resp)),
            None => (
                DnsProtocol::UDP,
                Err(UpstreamError::Other("invalid UDP response".into())),
            ),
        }
    }

    fn has_budget(&self, min: Duration) -> bool {
        self.request_budget.remaining().is_some_and(|r| r >= min)
    }

    /// Resolve the upstream request over TCP.
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

    /// Resolve the upstream request over UDP.
    async fn resolve_udp(&self, upstream: &Upstream, query: &[u8]) -> Result<Bytes, UpstreamError> {
        let deadline = self.request_budget.deadline();
        let mux = upstream
            .udp
            .as_ref()
            .ok_or_else(|| UpstreamError::Other(format!("upstream {} has no udp transport", upstream.addr)))?;

        mux.load().send_and_receive(query, deadline).await
    }
}
