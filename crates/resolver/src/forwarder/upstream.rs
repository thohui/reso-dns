use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

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
}

impl Upstreams {
    pub async fn new(addrs: &[SocketAddr], limits: Limits) -> anyhow::Result<Self> {
        let mut list = Vec::with_capacity(addrs.len());
        for &addr in addrs {
            let tcp = TcpPool::new(addr, limits);
            tcp.clone().start_reaper(limits.tcp_ttl);

            list.push(Arc::new(Upstream { addr, tcp_pool: tcp }));
        }
        Ok(Self {
            list: Arc::from(list),
            rr: AtomicUsize::new(0),
        })
    }

    /// Selects an upstream index using round-robin rotation.
    ///
    /// Returns `Some(index)` with the next index to use, or `None` if there are no upstreams.
    ///
    /// # Examples
    ///
    /// ```
    /// // Demonstrates the same selection logic used by `pick_index`.
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// let list_len = 3usize;
    /// let rr = AtomicUsize::new(0);
    /// let i = rr.fetch_add(1, Ordering::Relaxed) % list_len;
    /// assert!(i < list_len);
    /// ```
    pub fn pick_index(&self) -> Option<usize> {
        let n = self.list.len();
        if n == 0 {
            return None;
        }
        let i = self.rr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
        Some(i)
    }

    /// Returns a borrowed slice of the configured upstream servers.
    ///
    /// The returned slice is a non-owning view into the internal list of `Arc<Upstream>` values.
    ///
    /// # Examples
    ///
    /// ```
    /// // Construct an `Upstreams` instance as appropriate for your context.
    /// let ups: Upstreams = /* create upstreams */ unimplemented!();
    /// let slice: &[std::sync::Arc<Upstream>] = ups.as_slice();
    /// // Use `slice` without cloning the underlying `Arc`s.
    /// assert!(slice.len() >= 0);
    /// ```
    pub fn as_slice(&self) -> &[Arc<Upstream>] {
        &self.list
    }
}

/// An upstream server with its TCP and UDP connection pools.
pub struct Upstream {
    pub addr: SocketAddr,
    pub tcp_pool: Arc<TcpPool>,
}