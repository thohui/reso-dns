use std::{sync::Arc, time::Duration};

use super::{tcp::TcpPool, upstream::Upstreams};
use crate::{
    ResolveError, ResolveErrorKind,
    forwarder::upstream::{Upstream, UpstreamError},
};
use bytes::Bytes;
use reso_context::{DnsProtocol, RequestBudget};
use reso_dns::helpers;
use tokio::time::Instant;
use tracing::Instrument;

const MIN_REMAINING_TO_START_ATTEMPT: Duration = Duration::from_millis(15);
const MAX_ROUNDS: u32 = 3;
const ROUND_BACKOFF: Duration = Duration::from_millis(20);

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

    /// Resolve a DNS query by forwarding it to configured upstreams.
    pub async fn resolve(&self) -> Result<(Bytes, DnsProtocol), ResolveError> {
        let request_tid = helpers::extract_transaction_id(&self.query).ok_or(ResolveErrorKind::InvalidRequest(
            "failed to extract tid from query".into(),
        ))?;

        let req_type = self.request_type;

        // Protocol of the most recent upstream attempt, so failures can be attributed to a
        // transport. Stays `None` until an upstream is actually contacted.
        let mut last_protocol: Option<DnsProtocol> = None;

        for round in 0..MAX_ROUNDS {
            if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                return Err(ResolveErrorKind::Timeout.with_protocol(last_protocol));
            }

            let upstreams = self
                .upstreams
                .iter()
                .ok_or(ResolveErrorKind::Other("no upstreams available".into()))?;

            for (attempt, upstream) in upstreams.enumerate() {
                if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                    return Err(ResolveErrorKind::Timeout.with_protocol(last_protocol));
                }

                let span = tracing::debug_span!("upstream_attempt", upstream = %upstream.addr, round = round, attempt = attempt);

                let (protocol, attempt_res) = self.try_upstream(&upstream, req_type, round).instrument(span).await;
                last_protocol = Some(protocol);

                let resp = match attempt_res {
                    Ok(r) => {
                        upstream.health.record_success(upstream.addr);
                        r
                    }
                    Err(ref e) => {
                        // we only record failures for the first round.
                        if round == 0
                            && matches!(
                                e,
                                UpstreamError::SendTimeout
                                    | UpstreamError::RecvTimeout
                                    | UpstreamError::SendError(_)
                                    | UpstreamError::RecvError(_)
                                    | UpstreamError::RecvTaskStopped
                                    | UpstreamError::Tls(_)
                            )
                        {
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

            if round + 1 < MAX_ROUNDS && !self.backoff_before_next_round().await {
                return Err(ResolveErrorKind::Timeout.with_protocol(last_protocol));
            }
        }

        if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
            return Err(ResolveErrorKind::Timeout.with_protocol(last_protocol));
        }

        Err(ResolveErrorKind::Other("all upstreams failed".into()).with_protocol(last_protocol))
    }

    /// Attempt a single upstream. Returns the protocol that was actually used alongside the
    /// result, so a failed attempt can be attributed to a transport too.
    async fn try_upstream(
        &self,
        upstream: &Upstream,
        req_type: DnsProtocol,
        round: u32,
    ) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        let deadline = self.attempt_deadline(round);
        match req_type {
            DnsProtocol::TCP => self.try_tcp(&upstream.tcp, deadline).await,
            // A DoT upstream has no UDP mux, so it is forced to go over tcp (DoT).
            DnsProtocol::UDP if upstream.udp.is_none() => self.try_tcp(&upstream.tcp, deadline).await,
            DnsProtocol::UDP => self.resolve_udp_with_fallback(upstream, round).await,
            DnsProtocol::DOT | DnsProtocol::DOH => self.try_tcp(&upstream.tcp, deadline).await,
        }
    }

    /// Resolve over the tcp pool, reporting whether the connection was plain tcp or DoT.
    async fn try_tcp(&self, pool: &TcpPool, deadline: Instant) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        let protocol = if pool.has_tls() {
            DnsProtocol::DOT
        } else {
            DnsProtocol::TCP
        };
        (protocol, self.resolve_tcp(pool, &self.query, deadline).await)
    }

    async fn resolve_udp_with_fallback(
        &self,
        upstream: &Upstream,
        round: u32,
    ) -> (DnsProtocol, Result<Bytes, UpstreamError>) {
        let resp = match self
            .resolve_udp(upstream, &self.query, self.attempt_deadline(round))
            .await
        {
            Ok(resp) => resp,
            Err(e) => return (DnsProtocol::UDP, Err(e)),
        };

        match helpers::is_truncated(&resp) {
            Some(true) => {
                if !self.has_budget(MIN_REMAINING_TO_START_ATTEMPT) {
                    return (DnsProtocol::UDP, Err(UpstreamError::Timeout));
                }
                // TCP fallback for THIS upstream only, on a fresh slice: truncation is not a failure.
                self.try_tcp(&upstream.tcp, self.attempt_deadline(round)).await
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

    fn attempt_deadline(&self, round: u32) -> Instant {
        let Some(remaining) = self.request_budget.remaining() else {
            return self.request_budget.deadline();
        };
        let rounds_left = MAX_ROUNDS.saturating_sub(round).max(1);
        Instant::now() + remaining / rounds_left
    }

    async fn backoff_before_next_round(&self) -> bool {
        if !self.has_budget(ROUND_BACKOFF + MIN_REMAINING_TO_START_ATTEMPT) {
            return false;
        }
        tokio::time::sleep(ROUND_BACKOFF).await;
        true
    }

    /// Resolve the upstream request over tcp.
    async fn resolve_tcp(&self, pool: &TcpPool, query: &[u8], deadline: Instant) -> Result<Bytes, UpstreamError> {
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
    async fn resolve_udp(&self, upstream: &Upstream, query: &[u8], deadline: Instant) -> Result<Bytes, UpstreamError> {
        let mux = upstream
            .udp
            .as_ref()
            .ok_or_else(|| UpstreamError::Other(format!("upstream {} has no udp transport", upstream.addr)))?;

        mux.load().send_and_receive(query, deadline).await
    }
}
