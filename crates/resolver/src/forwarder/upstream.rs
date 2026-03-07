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

use super::tcp::TcpPool;

/// Limits for upstream connections.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Max total conns per upstream
    pub max_tcp_connections: usize,
    /// Idle conns to keep per upstream
    pub max_idle_tcp_connections: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// TCP connection time-to-live
    pub tcp_ttl: Duration,
}

/// List of upstream servers.
pub struct Upstreams {
    /// Upstream pools (1 per upstream server)
    list: Arc<[Arc<Upstream>]>,
    /// Round-robin index
    rr: AtomicUsize,
    /// Cached healthy upstream list, rebuilt periodically.
    healthy_cache: ArcSwap<Vec<Arc<Upstream>>>,
}

impl Upstreams {
    pub async fn new(addrs: &[SocketAddr], limits: Limits) -> Result<Self, std::io::Error> {
        let mut list = Vec::with_capacity(addrs.len());
        for &addr in addrs {
            let tcp = TcpPool::new(addr, limits);
            tcp.clone().start_reaper(limits.tcp_ttl);

            list.push(Arc::new(Upstream::new(addr, limits).await?));
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
                    Some(this) => this.rebuild_healthy_cache_if_dirty(),
                    None => return,
                }
            }
        });

        Ok(Arc::into_inner(upstreams).expect("no other references at construction"))
    }

    /// Returns the healthy upstreams and a round-robin starting index into that list.
    /// The caller should iterate over the returned slice starting at the given index.
    pub fn pick(&self) -> Option<(arc_swap::Guard<Arc<Vec<Arc<Upstream>>>>, usize)> {
        let upstreams = self.healthy_cache.load();
        let n = upstreams.len();
        if n == 0 {
            return None;
        }
        let i = self.rr.fetch_add(1, Ordering::Relaxed) % n;
        Some((upstreams, i))
    }

    fn rebuild_healthy_cache_if_dirty(&self) {
        // clear every upstream's dirty flag.
        let dirty = self.list.iter().fold(false, |acc, u| acc | u.take_dirty());
        if dirty {
            self.healthy_cache.store(Arc::new(Self::compute_healthy(&self.list)));
        }
    }

    pub fn rebuild_healthy_cache(&self) {
        self.healthy_cache.store(Arc::new(Self::compute_healthy(&self.list)));
    }

    fn compute_healthy(list: &Arc<[Arc<Upstream>]>) -> Vec<Arc<Upstream>> {
        let upstreams: Vec<_> = list.iter().filter(|u| u.is_healthy()).cloned().collect();
        // If no healthy upstreams, return all upstreams to allow requests to go through.
        if upstreams.is_empty() { list.to_vec() } else { upstreams }
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
    /// UDP mux for this upstream.
    pub udp: ArcSwap<UpstreamUdpMux>,
    /// TCP connection pool for this upstream.
    pub tcp: Arc<TcpPool>,
    /// Health status of the upstream, used to determine if it should be skipped for new requests.
    pub health: UpstreamHealth,
    /// Set when health changes so the healthy cache gets rebuilt on the next tick.
    dirty: AtomicBool,
    /// Flag to prevent concurrent UDP reconnect attempts.
    udp_reconnecting: AtomicBool,
}

impl Upstream {
    pub async fn new(addr: SocketAddr, limits: Limits) -> Result<Self, std::io::Error> {
        let tcp = TcpPool::new(addr, limits);
        tcp.clone().start_reaper(limits.tcp_ttl);

        Ok(Self {
            addr,
            tcp,
            udp: ArcSwap::from_pointee(UpstreamUdpMux::new(addr).await?),
            health: UpstreamHealth::new(),
            dirty: AtomicBool::new(false),
            udp_reconnecting: AtomicBool::new(false),
        })
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

    pub fn record_success(&self) {
        self.health.record_success(self.addr);
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.health.record_failure(self.addr);
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    pub fn trigger_udp_reconnect(self: Arc<Self>) {
        if self.udp_reconnecting.swap(true, Ordering::AcqRel) {
            return;
        }
        tokio::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            loop {
                tokio::time::sleep(backoff).await;
                match UpstreamUdpMux::new(self.addr).await {
                    Ok(mux) => {
                        self.udp.store(Arc::new(mux));
                        self.udp_reconnecting.store(false, Ordering::Release);
                        tracing::info!(upstream = %self.addr, "UDP mux reconnected");
                        return;
                    }
                    Err(e) => {
                        tracing::warn!(upstream = %self.addr, error = %e, "UDP reconnect failed, retrying");
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
    #[error("upstream error: {0}")]
    Other(String),
}

impl From<UpstreamError> for crate::ResolveError {
    fn from(e: UpstreamError) -> Self {
        match e {
            UpstreamError::SendTimeout | UpstreamError::RecvTimeout => crate::ResolveError::Timeout,
            other => crate::ResolveError::Other(other.to_string()),
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

    #[tokio::test]
    async fn pick_round_robin() {
        let addrs: Vec<SocketAddr> = vec!["127.0.0.1:5353".parse().unwrap(), "127.0.0.2:5353".parse().unwrap()];
        let upstreams = Upstreams::new(&addrs, test_limits()).await.unwrap();

        let (list1, idx1) = upstreams.pick().unwrap();
        let (list2, idx2) = upstreams.pick().unwrap();

        assert_eq!(list1.len(), 2);
        assert_eq!(list2.len(), 2);
        // Round-robin should advance
        assert_ne!(idx1, idx2);
    }

    #[tokio::test]
    async fn pick_skips_unhealthy() {
        let addrs: Vec<SocketAddr> = vec!["127.0.0.1:5353".parse().unwrap(), "127.0.0.2:5353".parse().unwrap()];
        let upstreams = Upstreams::new(&addrs, test_limits()).await.unwrap();

        // make first upstream unhealthy
        let addr = upstreams.list[0].addr;
        for _ in 0..UpstreamHealth::FAILURE_THRESHOLD {
            upstreams.list[0].health.record_failure(addr);
        }

        // rebuild cache so pick sees the change
        upstreams.rebuild_healthy_cache();

        let (list, _) = upstreams.pick().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].addr, addrs[1]);
    }

    #[tokio::test]
    async fn pick_returns_all_when_all_unhealthy() {
        let addrs: Vec<SocketAddr> = vec!["127.0.0.1:5353".parse().unwrap(), "127.0.0.2:5353".parse().unwrap()];
        let upstreams = Upstreams::new(&addrs, test_limits()).await.unwrap();

        // Make all upstreams unhealthy
        for upstream in upstreams.list.iter() {
            for _ in 0..UpstreamHealth::FAILURE_THRESHOLD {
                upstream.health.record_failure(upstream.addr);
            }
        }

        upstreams.rebuild_healthy_cache();

        let (list, _) = upstreams.pick().unwrap();
        assert_eq!(list.len(), 2);
    }
}
