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

            list.push(Arc::new(Upstream {
                addr,
                tcp_pool: tcp,
            }));
        }
        Ok(Self {
            list: Arc::from(list),
            rr: AtomicUsize::new(0),
        })
    }

    ///  Pick an upstream index in round-robin fashion.
    pub fn pick_index(&self) -> Option<usize> {
        let n = self.list.len();
        if n == 0 {
            return None;
        }
        let i = self.rr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
        Some(i)
    }

    /// Pick an upstream in round-robin fashion.
    pub fn pick(&self) -> Option<Arc<Upstream>> {
        let index = self.pick_index()?;
        Some(Arc::clone(&self.list[index]))
    }

    /// Get the list of upstreams as a slice.
    pub fn as_slice(&self) -> &[Arc<Upstream>] {
        &self.list
    }
}

/// An upstream server with its TCP and UDP connection pools.
pub struct Upstream {
    pub addr: SocketAddr,
    pub tcp_pool: Arc<TcpPool>,
}
