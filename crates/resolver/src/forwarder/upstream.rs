use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, anyhow};

use bytes::{Bytes, BytesMut};
use crossbeam_queue::SegQueue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    sync::{Mutex, OwnedSemaphorePermit, Semaphore},
    time::{Instant, timeout, timeout_at},
};

/// Limits for upstream connections.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Max total conns per upstream
    pub max_total: usize,
    /// Idle conns to keep per upstream
    pub max_idle: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// TCP connection time-to-live
    pub tcp_ttl: Duration,
    /// Number of UDP sockets to create per upstream
    pub udp_sockets: usize,
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

            let udp = UdpPool::new(addr, limits, limits.udp_sockets).await?;

            list.push(Arc::new(Upstream { addr, tcp, udp }));
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
    pub tcp: Arc<TcpPool>,
    pub udp: Arc<UdpPool>,
}

/// Maximum receive buffer size.
const MAX_RECEIVE_BUFFER_SIZE: usize = 65_536;

/// A pool of UDP connections to a specific upstream server.
pub struct UdpPool {
    /// Upstream address
    pub limits: Limits,
    pub sockets: Vec<Arc<UdpConn>>,
    /// Round robin idx
    pub rr: AtomicUsize,
}

impl UdpPool {
    pub async fn new(
        addr: SocketAddr,
        limits: Limits,
        n_sockets: usize,
    ) -> anyhow::Result<Arc<Self>> {
        if n_sockets == 0 {
            anyhow::bail!("n_sockets must be > 0");
        }
        let mut sockets = Vec::with_capacity(n_sockets);
        for _ in 0..n_sockets {
            let s = UdpConn::new(addr).await?;
            sockets.push(Arc::new(s));
        }
        Ok(Self {
            limits,
            sockets,
            rr: AtomicUsize::new(0),
        }
        .into())
    }

    /// Pick a UDP connection index in round-robin fashion.
    pub fn pick_index(&self) -> usize {
        let n = self.sockets.len();
        self.rr.fetch_add(1, Ordering::Relaxed) % n
    }

    /// Pick a UDP connection in round-robin fashion.
    pub fn pick(&self) -> Arc<UdpConn> {
        let idx = self.pick_index();
        Arc::clone(&self.sockets[idx])
    }

    pub fn pick_seeded(&self, seed: usize) -> Arc<UdpConn> {
        let n = self.sockets.len();
        let idx = seed % n;
        Arc::clone(&self.sockets[idx])
    }
}

/// A single UDP connection to an upstream server.
#[derive(Debug)]
pub struct UdpConn {
    /// Shared UDP socket
    pub socket: UdpSocket,
    /// Guard to ensure only one query at a time per UDP socket
    pub guard: Arc<Semaphore>,
    /// Reusable buffer for receiving data
    pub buffer: Mutex<BytesMut>,
}

impl UdpConn {
    pub async fn new(upstream_addr: SocketAddr) -> anyhow::Result<Self> {
        let bind_addr = if upstream_addr.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from(([0u16; 8], 0))
        };
        let socket = UdpSocket::bind(bind_addr).await?;
        socket.connect(upstream_addr).await?;
        Ok(Self {
            buffer: Mutex::new(BytesMut::with_capacity(MAX_RECEIVE_BUFFER_SIZE)),
            socket,
            guard: Arc::new(Semaphore::new(1)),
        })
    }

    pub async fn send_and_receive(&self, query: &[u8], deadline: Instant) -> anyhow::Result<Bytes> {
        // check query size
        if query.len() > u16::MAX as usize {
            anyhow::bail!("query too large for DNS/UDP: {}", query.len());
        }
        let want_id = u16::from_be_bytes([query[0], query[1]]);

        // ensure only one query at a time
        let _permit = self.guard.clone().acquire_owned().await?;

        timeout_at(deadline, self.socket.send(query))
            .await
            .context("send timeout")??;

        let mut buf = vec![0u8; MAX_RECEIVE_BUFFER_SIZE];

        // this loop is needed to ignore stale/foreign packets
        loop {
            let n = tokio::time::timeout_at(deadline, self.socket.recv(&mut buf))
                .await
                .context("recv timeout")??;

            if n >= 12 {
                let got_id = u16::from_be_bytes([buf[0], buf[1]]);
                let qr = (buf[2] & 0x80) != 0;
                if qr && got_id == want_id {
                    buf.truncate(n);
                    return Ok(Bytes::from(buf));
                }
            }
        }
    }
}

/// A pool of TCP connections to a specific upstream server.
/// Existing connections are reused if possible, otherwise new connections are created
pub struct TcpPool {
    /// Upstream address
    pub addr: SocketAddr,
    /// Upstream limits
    pub limits: Limits,
    /// Idle connections
    idle: SegQueue<TcpConn>,
    /// Count of idle connections
    idle_count: AtomicUsize,
    /// Total connections (including in-use and connecting)
    connections: Arc<Semaphore>,
}

impl TcpPool {
    fn new(addr: SocketAddr, limits: Limits) -> Arc<Self> {
        Arc::new(Self {
            addr,
            limits,
            idle: SegQueue::new(),
            idle_count: AtomicUsize::new(0),
            connections: Arc::new(Semaphore::new(limits.max_total)),
        })
    }

    /// Start a background task that reaps expired idle tcp connections.
    pub fn start_reaper(self: Arc<Self>, interval: Duration) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let mut dropped = 0;
                for _ in 0..this.idle_count.load(Ordering::Relaxed) {
                    if let Some(conn) = this.idle.pop() {
                        if conn.ttl > now {
                            this.idle.push(conn);
                        } else {
                            dropped += 1;
                            this.idle_count.fetch_sub(1, Ordering::Relaxed);
                            drop(conn);
                        }
                    } else {
                        break;
                    }
                }
                if dropped > 0 {
                    tracing::info!(
                        "reaper dropped {} expired tcp conns to {}",
                        dropped,
                        this.addr
                    );
                }
            }
        });
    }

    /// Try to get an idle conn.
    pub fn try_get(&self) -> Option<TcpConn> {
        if let Some(conn) = self.idle.pop() {
            self.idle_count.fetch_sub(1, Ordering::Relaxed);
            Some(conn)
        } else {
            None
        }
    }

    /// Wait for tcp connection to become available or timeout.
    pub async fn wait_checkout(&self, overall: Duration) -> Option<TcpConn> {
        // check if we have one available right now.
        if let Some(c) = self.try_get() {
            return Some(c);
        }
        let connections = self.connections.clone();
        let permit = timeout(overall, connections.acquire_owned())
            .await
            .ok()?
            .ok()?;

        let to = self.limits.connect_timeout.min(overall);

        TcpConn::connect(self.addr, to, permit, Instant::now() + self.limits.tcp_ttl)
            .await
            .ok()
    }

    /// Get an idle conn or connect a new one if under cap.
    pub async fn get_or_connect(&self, deadline: Instant) -> anyhow::Result<TcpConn> {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(deadline) => Err(anyhow!("deadline reached")),
            res = self.get_or_connect_inner() => res
        }
    }

    async fn get_or_connect_inner(&self) -> anyhow::Result<TcpConn> {
        if let Some(c) = self.try_get() {
            return Ok(c);
        }

        let permit = self.connections.clone().try_acquire_owned().map_err(|_| {
            anyhow!(
                "upstream {} at max concurrent connection attempts",
                self.addr
            )
        })?;

        TcpConn::connect(
            self.addr,
            self.limits.connect_timeout,
            permit,
            Instant::now() + self.limits.tcp_ttl,
        )
        .await
    }

    /// Attempt to put back a connection to the pool.
    pub fn put_back(&self, conn: TcpConn, healthy: bool) {
        if healthy && self.idle_count.load(Ordering::Relaxed) < self.limits.max_idle {
            self.idle.push(conn);
            self.idle_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// A single TCP connection to an upstream server.
pub struct TcpConn {
    /// The TCP stream
    stream: TcpStream,
    /// Permit that keeps the connection slot
    _permit: OwnedSemaphorePermit,
    /// Time-to-live for this connection
    pub ttl: Instant,
    /// Reusable buffer for receiving data
    buffer: BytesMut,
}

impl TcpConn {
    /// Establish a new TCP connection to the given address with a timeout and a permit.
    async fn connect(
        addr: SocketAddr,
        to: Duration,
        _permit: OwnedSemaphorePermit,
        ttl: Instant,
    ) -> anyhow::Result<Self> {
        let s = timeout(to, TcpStream::connect(addr))
            .await
            .context("tcp connect timeout")??;

        // this allows us to avoid delays in sending small packets, which we are doing in the send_and_receive method.
        s.set_nodelay(true)?;

        Ok(Self {
            stream: s,
            _permit,
            ttl,
            buffer: BytesMut::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
        })
    }

    /// Send a DNS query and receive the response over this TCP connection.
    pub async fn send_and_receive(
        &mut self,
        query: &[u8],
        deadline: Instant,
    ) -> anyhow::Result<Bytes> {
        if query.len() > u16::MAX as usize {
            anyhow::bail!("query too large for DNS/TCP: {}", query.len());
        }

        // write length + body
        // should be fine to write these two separately as they are small and we set tcp_nodelay
        let lenb = (query.len() as u16).to_be_bytes();
        timeout_at(deadline, self.stream.write_all(&lenb))
            .await
            .context("write len timeout")??;

        timeout_at(deadline, self.stream.write_all(query))
            .await
            .context("write body timeout")??;

        // read resp
        let mut resp_lenb = [0u8; 2];
        timeout_at(deadline, self.stream.read_exact(&mut resp_lenb))
            .await
            .context("read len timeout")??;
        let n = u16::from_be_bytes(resp_lenb) as usize;

        if self.buffer.capacity() < n {
            self.buffer.reserve(n - self.buffer.capacity());
        }

        self.buffer.resize(n, 0);

        timeout_at(deadline, self.stream.read_exact(&mut self.buffer[..]))
            .await
            .context("read body timeout")??;

        let resp = self.buffer.split().freeze();
        Ok(resp)
    }
}
