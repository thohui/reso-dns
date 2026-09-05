use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};

use arc_swap::ArcSwap;

use crate::forwarder::udp::UpstreamUdpMux;

use super::{dot::TlsUpstream, tcp::TcpPool};

/// Limits for upstream connections.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_tcp_connections: usize,
    pub max_idle_tcp_connections: usize,
    pub connect_timeout: Duration,
    pub tcp_ttl: Duration,
}

/// List of upstream servers.
pub struct Upstreams {
    /// Upstream pools (1 per upstream server)
    list: Arc<[Arc<Upstream>]>,
    rr: AtomicUsize,
    healthy_cache: ArcSwap<Vec<Arc<Upstream>>>,
}

impl Upstreams {
    pub async fn new(configs: &[crate::Upstream], limits: Limits) -> Result<Arc<Self>, UpstreamError> {
        let mut list = Vec::with_capacity(configs.len());

        let mut last_err: Option<UpstreamError> = None;

        for config in configs {
            match Upstream::from_config(config, limits).await {
                Ok(upstream) => {
                    list.push(Arc::new(upstream));
                }
                Err(err) => {
                    tracing::error!(upstream = ?config, error = %err, "skipping unusable upstream");
                    last_err = Some(err);
                }
            }
        }

        if list.is_empty()
            && let Some(last_err) = last_err
        {
            return Err(last_err);
        }

        let list: Arc<[Arc<Upstream>]> = Arc::from(list);
        let initial_healthy = Self::compute_healthy(&list);
        let upstreams = Arc::new(Self {
            list,
            rr: AtomicUsize::new(0),
            healthy_cache: ArcSwap::from_pointee(initial_healthy),
        });

        // spawn periodic rebuild task using Weak to avoid leaking.
        let weak = Arc::downgrade(&upstreams);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(1));
            loop {
                ticker.tick().await;
                match weak.upgrade() {
                    Some(this) => this.rebuild_healthy_cache(),
                    None => return,
                }
            }
        });

        Ok(upstreams)
    }

    pub fn iter(&self) -> Option<UpstreamIter> {
        let upstreams = self.healthy_cache.load_full();
        let n = upstreams.len();
        if n == 0 {
            return None;
        }
        let starting_index = self.rr.fetch_add(1, Ordering::Relaxed) % n;

        Some(UpstreamIter {
            upstreams,
            start: starting_index,
            offset: 0,
        })
    }

    pub fn rebuild_healthy_cache(&self) {
        self.healthy_cache.store(Arc::new(Self::compute_healthy(&self.list)));
    }

    fn compute_healthy(list: &Arc<[Arc<Upstream>]>) -> Vec<Arc<Upstream>> {
        let upstreams: Vec<_> = list.iter().filter(|u| u.is_healthy()).cloned().collect();
        if upstreams.is_empty() { list.to_vec() } else { upstreams }
    }
}

pub struct UpstreamIter {
    upstreams: Arc<Vec<Arc<Upstream>>>,
    start: usize,
    offset: usize,
}

impl Iterator for UpstreamIter {
    type Item = Arc<Upstream>;
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.upstreams.len();
        if self.offset >= n {
            return None;
        }
        let idx = (self.start + self.offset) % n;
        self.offset += 1;
        Some(Arc::clone(&self.upstreams[idx]))
    }
}

#[derive(Debug)]
pub struct UpstreamHealth {
    consecutive_failures: AtomicU32,
    skip_until: AtomicU64, // timestamp in milliseconds until which this upstream should be skipped due to unhealthy status. 0 = not skipped.
}

impl UpstreamHealth {
    /// Number of consecutive failures to consider an upstream unhealthy and start skipping it.
    const FAILURE_THRESHOLD: u32 = 5;
    /// Base cooldown duration in milliseconds to skip an unhealthy upstream.
    const BASE_COOLDOWN_MS: u64 = 2000;
    /// Maximum cooldown duration in milliseconds when skipping an unhealthy upstream.
    const MAX_COOLDOWN_MS: u64 = 30000;

    pub fn new() -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            skip_until: AtomicU64::new(0),
        }
    }

    fn cooldown_ms(failures: u32) -> u64 {
        if failures < Self::FAILURE_THRESHOLD {
            0
        } else {
            let cooldown =
                Self::BASE_COOLDOWN_MS.saturating_mul(2u64.saturating_pow(failures - Self::FAILURE_THRESHOLD));
            cooldown.min(Self::MAX_COOLDOWN_MS)
        }
    }

    pub fn record_success(&self, addr: SocketAddr) {
        let prev_failures = self.consecutive_failures.swap(0, Ordering::Relaxed);
        let was_unhealthy = prev_failures >= Self::FAILURE_THRESHOLD;
        self.skip_until.store(0, Ordering::Relaxed);
        if was_unhealthy {
            tracing::info!(upstream = %addr, prev_failures, "upstream recovered");
        }
    }

    pub fn record_failure(&self, addr: SocketAddr) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= Self::FAILURE_THRESHOLD {
            let cooldown = Self::cooldown_ms(failures);
            let current_time_ms = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let skip_until = current_time_ms.saturating_add(cooldown);
            self.skip_until.store(skip_until, Ordering::Relaxed);
            if failures == Self::FAILURE_THRESHOLD {
                tracing::warn!(upstream = %addr, failures, cooldown_ms = cooldown, "upstream became unhealthy");
            }
        }
    }
}

/// An upstream server with its TCP and UDP connection pools.
pub struct Upstream {
    /// Address of the upstream server.
    pub addr: SocketAddr,
    /// UDP mux for this upstream, is None when the upstream is DoT
    pub udp: Option<ArcSwap<UpstreamUdpMux>>,
    /// Connection pool for this upstream, TLS-wrapped when the upstream is DoT.
    pub tcp: Arc<TcpPool>,
    /// Health status of the upstream, used to determine if it should be skipped for new requests.
    pub health: UpstreamHealth,
    /// Flag to prevent concurrent UDP reconnect attempts.
    udp_reconnecting: AtomicBool,
}

impl Upstream {
    async fn from_config(config: &crate::Upstream, limits: Limits) -> Result<Self, UpstreamError> {
        match config {
            crate::Upstream::Plain { endpoint } => Self::plain(*endpoint, limits).await,
            crate::Upstream::Tls { endpoint, hostname } => Self::tls(*endpoint, hostname.as_deref(), limits),
        }
    }

    /// Plain DNS upstream (UDP or TCP)
    pub async fn plain(addr: SocketAddr, limits: Limits) -> Result<Self, UpstreamError> {
        let udp = UpstreamUdpMux::new(addr)
            .await
            .map_err(|e| UpstreamError::Other(format!("udp socket setup for {addr} failed: {e}")))?;

        Ok(Self::build(addr, Some(ArcSwap::from_pointee(udp)), None, limits))
    }

    /// DNS over TLS upstream. `hostname` is the certificate identity, not an address.
    pub fn tls(addr: SocketAddr, hostname: Option<&str>, limits: Limits) -> Result<Self, UpstreamError> {
        let tls = Arc::new(TlsUpstream::new(addr, hostname)?);

        Ok(Self::build(addr, None, Some(tls), limits))
    }

    fn build(
        addr: SocketAddr,
        udp: Option<ArcSwap<UpstreamUdpMux>>,
        tls: Option<Arc<TlsUpstream>>,
        limits: Limits,
    ) -> Self {
        let tcp = TcpPool::new(addr, limits, tls);
        tcp.clone().start_reaper(limits.tcp_ttl.max(Duration::from_secs(1)));

        Self {
            addr,
            tcp,
            udp,
            health: UpstreamHealth::new(),
            udp_reconnecting: AtomicBool::new(false),
        }
    }

    pub fn is_healthy(&self) -> bool {
        let skip_until = self.health.skip_until.load(Ordering::Relaxed);
        if skip_until == 0 {
            true
        } else {
            let current_time_ms = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            current_time_ms >= skip_until
        }
    }

    pub fn trigger_udp_reconnect(self: Arc<Self>) {
        // DoT upstreams have no mux to rebuild.
        if self.udp.is_none() {
            return;
        }
        if self.udp_reconnecting.swap(true, Ordering::AcqRel) {
            return;
        }
        tokio::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            const MAX_RETRIES: u32 = 10;
            let mut retries = 0;

            loop {
                tokio::time::sleep(backoff).await;
                match UpstreamUdpMux::new(self.addr).await {
                    Ok(mux) => {
                        if let Some(udp) = self.udp.as_ref() {
                            udp.store(Arc::new(mux));
                        }
                        self.udp_reconnecting.store(false, Ordering::Release);
                        tracing::info!(upstream = %self.addr, "UDP mux reconnected");
                        return;
                    }
                    Err(e) => {
                        retries += 1;
                        if retries >= MAX_RETRIES {
                            tracing::error!(upstream = %self.addr, "UDP mux reconnect failed after {} retries, giving up", MAX_RETRIES);
                            self.udp_reconnecting.store(false, Ordering::Release);
                            return;
                        }
                        tracing::warn!(upstream = %self.addr, error = %e, "UDP mux reconnect failed, retrying");
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                    }
                }
            }
        });
    }
}

#[derive(thiserror::Error, Debug)]
pub enum UpstreamError {
    #[error("upstream request timed out")]
    Timeout,
    #[error("upstream send timeout")]
    SendTimeout,
    #[error("upstream recv timeout")]
    RecvTimeout,
    #[error("upstream recv task stopped")]
    RecvTaskStopped,
    #[error("upstream send error: {0}")]
    SendError(std::io::Error),
    #[error("upstream recv error: {0}")]
    RecvError(std::io::Error),
    /// TLS setup or handshake failure. Kept separate from the transport errors above
    /// because it is almost always a misconfiguration, not a transient network fault.
    #[error("upstream tls error: {0}")]
    Tls(String),
    #[error("upstream error: {0}")]
    Other(String),
}

impl From<UpstreamError> for crate::ResolveErrorKind {
    fn from(e: UpstreamError) -> Self {
        match e {
            UpstreamError::SendTimeout | UpstreamError::RecvTimeout => crate::ResolveErrorKind::Timeout,
            other => crate::ResolveErrorKind::Other(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_limits() -> Limits {
        Limits {
            max_tcp_connections: 10,
            max_idle_tcp_connections: 5,
            connect_timeout: Duration::from_secs(5),
            tcp_ttl: Duration::from_secs(30),
        }
    }

    fn plain(endpoint: &str) -> crate::Upstream {
        crate::Upstream::Plain {
            endpoint: endpoint.parse().expect("valid endpoint"),
        }
    }

    #[tokio::test]
    async fn iter_round_robin() {
        let configs = [plain("127.0.0.1:5353"), plain("127.0.0.2:5353")];
        let upstreams = Upstreams::new(&configs, test_limits()).await.unwrap();

        let first = upstreams.iter().unwrap().next().unwrap();
        let second = upstreams.iter().unwrap().next().unwrap();

        assert_ne!(first.addr, second.addr);
    }

    #[tokio::test]
    async fn iter_skips_unhealthy() {
        let configs = [plain("127.0.0.1:5353"), plain("127.0.0.2:5353")];
        let upstreams = Upstreams::new(&configs, test_limits()).await.unwrap();

        let addr = upstreams.list[0].addr;
        for _ in 0..UpstreamHealth::FAILURE_THRESHOLD {
            upstreams.list[0].health.record_failure(addr);
        }
        upstreams.rebuild_healthy_cache();

        let results: Vec<_> = upstreams.iter().unwrap().collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].addr, "127.0.0.2:5353".parse::<SocketAddr>().unwrap());
    }

    #[tokio::test]
    async fn iter_returns_all_when_all_unhealthy() {
        let configs = [plain("127.0.0.1:5353"), plain("127.0.0.2:5353")];
        let upstreams = Upstreams::new(&configs, test_limits()).await.unwrap();

        for upstream in upstreams.list.iter() {
            for _ in 0..UpstreamHealth::FAILURE_THRESHOLD {
                upstream.health.record_failure(upstream.addr);
            }
        }
        upstreams.rebuild_healthy_cache();

        let results: Vec<_> = upstreams.iter().unwrap().collect();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn tls_upstream_has_no_udp_mux() {
        let configs = [crate::Upstream::Tls {
            endpoint: "9.9.9.9:853".parse().unwrap(),
            hostname: Some("dns.quad9.net".into()),
        }];
        let upstreams = Upstreams::new(&configs, test_limits()).await.unwrap();

        assert!(upstreams.list[0].udp.is_none());
    }

    #[tokio::test]
    async fn plain_upstream_has_a_udp_mux() {
        let upstreams = Upstreams::new(&[plain("127.0.0.1:5353")], test_limits()).await.unwrap();

        assert!(upstreams.list[0].udp.is_some());
    }

    #[tokio::test]
    async fn rejects_tls_upstream_with_invalid_hostname() {
        let configs = [crate::Upstream::Tls {
            endpoint: "9.9.9.9:853".parse().unwrap(),
            hostname: Some("not a hostname".into()),
        }];

        assert!(Upstreams::new(&configs, test_limits()).await.is_err());
    }

    #[tokio::test]
    async fn health_rebuild_task_keeps_running_after_construction() {
        let configs = [plain("127.0.0.1:5353"), plain("127.0.0.2:5353")];
        let upstreams = Upstreams::new(&configs, test_limits()).await.unwrap();

        let addr = upstreams.list[0].addr;
        for _ in 0..UpstreamHealth::FAILURE_THRESHOLD {
            upstreams.list[0].health.record_failure(addr);
        }

        tokio::time::sleep(Duration::from_millis(1200)).await;

        let results: Vec<_> = upstreams.iter().unwrap().collect();
        assert_eq!(results.len(), 1, "unhealthy upstream was not dropped from the cache");
        assert_eq!(results[0].addr, "127.0.0.2:5353".parse::<SocketAddr>().unwrap());
    }
}
