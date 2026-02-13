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

    pub fn pick_index(&self) -> Option<usize> {
        let n = self.list.len();
        if n == 0 {
            return None;
        }
        let i = self.rr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n;
        Some(i)
    }

    pub fn as_slice(&self) -> &[Arc<Upstream>] {
        &self.list
    }
}

/// An upstream server with its TCP and UDP connection pools.
pub struct Upstream {
    pub addr: SocketAddr,
    pub tcp_pool: Arc<TcpPool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_limits() -> Limits {
        Limits {
            max_tcp_connections: 10,
            max_idle_tcp_connections: 5,
            connect_timeout: Duration::from_secs(5),
            tcp_ttl: Duration::from_secs(30),
        }
    }

    fn create_test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
    }

    #[tokio::test]
    async fn test_upstreams_creation_empty() {
        let limits = create_test_limits();
        let upstreams = Upstreams::new(&[], limits).await.unwrap();

        assert_eq!(upstreams.as_slice().len(), 0);
        assert!(upstreams.pick_index().is_none());
    }

    #[tokio::test]
    async fn test_upstreams_creation_single() {
        let limits = create_test_limits();
        let addrs = vec![create_test_addr(53)];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        assert_eq!(upstreams.as_slice().len(), 1);
        assert_eq!(upstreams.as_slice()[0].addr, addrs[0]);
    }

    #[tokio::test]
    async fn test_upstreams_creation_multiple() {
        let limits = create_test_limits();
        let addrs = vec![
            create_test_addr(53),
            create_test_addr(5353),
            create_test_addr(8853),
        ];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        assert_eq!(upstreams.as_slice().len(), 3);
        for (i, upstream) in upstreams.as_slice().iter().enumerate() {
            assert_eq!(upstream.addr, addrs[i]);
        }
    }

    #[tokio::test]
    async fn test_pick_index_single_upstream() {
        let limits = create_test_limits();
        let addrs = vec![create_test_addr(53)];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        // Should always return 0 for single upstream
        for _ in 0..10 {
            assert_eq!(upstreams.pick_index(), Some(0));
        }
    }

    #[tokio::test]
    async fn test_pick_index_round_robin() {
        let limits = create_test_limits();
        let addrs = vec![
            create_test_addr(53),
            create_test_addr(5353),
            create_test_addr(8853),
        ];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        // Should cycle through 0, 1, 2, 0, 1, 2, ...
        let mut indices = Vec::new();
        for _ in 0..9 {
            indices.push(upstreams.pick_index().unwrap());
        }

        assert_eq!(indices, vec![0, 1, 2, 0, 1, 2, 0, 1, 2]);
    }

    #[tokio::test]
    async fn test_pick_index_two_upstreams() {
        let limits = create_test_limits();
        let addrs = vec![create_test_addr(53), create_test_addr(5353)];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        // Should alternate between 0 and 1
        let mut indices = Vec::new();
        for _ in 0..6 {
            indices.push(upstreams.pick_index().unwrap());
        }

        assert_eq!(indices, vec![0, 1, 0, 1, 0, 1]);
    }

    #[test]
    fn test_limits_values() {
        let limits = Limits {
            max_tcp_connections: 100,
            max_idle_tcp_connections: 50,
            connect_timeout: Duration::from_secs(10),
            tcp_ttl: Duration::from_secs(60),
        };

        assert_eq!(limits.max_tcp_connections, 100);
        assert_eq!(limits.max_idle_tcp_connections, 50);
        assert_eq!(limits.connect_timeout, Duration::from_secs(10));
        assert_eq!(limits.tcp_ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_limits_clone() {
        let limits = create_test_limits();
        let cloned = limits.clone();

        assert_eq!(limits.max_tcp_connections, cloned.max_tcp_connections);
        assert_eq!(
            limits.max_idle_tcp_connections,
            cloned.max_idle_tcp_connections
        );
        assert_eq!(limits.connect_timeout, cloned.connect_timeout);
        assert_eq!(limits.tcp_ttl, cloned.tcp_ttl);
    }

    #[tokio::test]
    async fn test_upstream_addr() {
        let limits = create_test_limits();
        let addr = create_test_addr(53);
        let addrs = vec![addr];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();
        let upstream = &upstreams.as_slice()[0];

        assert_eq!(upstream.addr, addr);
        assert_eq!(upstream.addr.port(), 53);
        assert_eq!(upstream.addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    #[tokio::test]
    async fn test_upstreams_as_slice() {
        let limits = create_test_limits();
        let addrs = vec![create_test_addr(53), create_test_addr(5353)];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();
        let slice = upstreams.as_slice();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].addr, addrs[0]);
        assert_eq!(slice[1].addr, addrs[1]);
    }

    #[tokio::test]
    async fn test_pick_index_wraps_around() {
        let limits = create_test_limits();
        let addrs = vec![create_test_addr(53), create_test_addr(5353)];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        // Pick many times to ensure wrapping works correctly
        for i in 0..100 {
            let idx = upstreams.pick_index().unwrap();
            assert_eq!(idx, i % 2);
        }
    }

    #[tokio::test]
    async fn test_upstreams_with_ipv6() {
        use std::net::Ipv6Addr;

        let limits = create_test_limits();
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 53);
        let addrs = vec![addr];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        assert_eq!(upstreams.as_slice().len(), 1);
        assert_eq!(upstreams.as_slice()[0].addr, addr);
    }

    #[tokio::test]
    async fn test_upstreams_mixed_ip_versions() {
        use std::net::Ipv6Addr;

        let limits = create_test_limits();
        let ipv4_addr = create_test_addr(53);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 53);
        let addrs = vec![ipv4_addr, ipv6_addr];

        let upstreams = Upstreams::new(&addrs, limits).await.unwrap();

        assert_eq!(upstreams.as_slice().len(), 2);
        assert_eq!(upstreams.as_slice()[0].addr, ipv4_addr);
        assert_eq!(upstreams.as_slice()[1].addr, ipv6_addr);
    }

    #[tokio::test]
    async fn test_limits_copy() {
        let limits1 = create_test_limits();
        let limits2 = limits1; // Should copy, not move

        // Both should be usable
        assert_eq!(limits1.max_tcp_connections, 10);
        assert_eq!(limits2.max_tcp_connections, 10);
    }
}