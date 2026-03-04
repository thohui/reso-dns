use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::OwnedSemaphorePermit,
    time::{Duration, Instant, timeout_at},
};

use crossbeam_queue::SegQueue;
use tokio::sync::Semaphore;

use super::upstream::{Limits, UpstreamError};

/// A pool of TCP connections to a specific upstream server.
/// Existing connections are reused if possible, otherwise new connections are created
pub(crate) struct TcpPool {
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
    pub fn new(addr: SocketAddr, limits: Limits) -> Arc<Self> {
        Arc::new(Self {
            addr,
            limits,
            idle: SegQueue::new(),
            idle_count: AtomicUsize::new(0),
            connections: Arc::new(Semaphore::new(limits.max_tcp_connections)),
        })
    }

    /// Start a background task that reaps expired idle tcp connections.
    pub fn start_reaper(self: Arc<Self>, interval: Duration) {
        // Use a weak reference to avoid keeping the pool alive if it is dropped.
        let weak = Arc::downgrade(&self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let this = match weak.upgrade() {
                    Some(pool) => pool,
                    None => return,
                };
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
                    tracing::debug!("reaper dropped {} expired tcp conns to {}", dropped, this.addr);
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

    /// Get an idle conn or connect a new one if under cap.
    pub async fn get_or_connect(&self, deadline: Instant) -> Result<TcpConn, UpstreamError> {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(deadline) => Err(UpstreamError::SendTimeout),
            res = self.get_or_connect_inner(deadline) => res
        }
    }

    async fn get_or_connect_inner(&self, deadline: Instant) -> Result<TcpConn, UpstreamError> {
        if let Some(c) = self.try_get() {
            tracing::debug!(upstream = %self.addr, "reusing idle tcp connection");
            return Ok(c);
        }

        let permit = self.connections.clone().try_acquire_owned().map_err(|_| {
            UpstreamError::Other(format!("upstream {} at max concurrent connection attempts", self.addr))
        })?;

        tracing::debug!(upstream = %self.addr, "opening new tcp connection");

        TcpConn::connect(
            self.addr,
            deadline,
            self.limits.connect_timeout,
            permit,
            Instant::now() + self.limits.tcp_ttl,
        )
        .await
    }

    /// Attempt to put back a connection to the pool.
    pub fn put_back(&self, conn: TcpConn, healthy: bool) {
        if healthy && self.idle_count.load(Ordering::Relaxed) < self.limits.max_idle_tcp_connections {
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
    /// The effective timeout is `min(now + connect_timeout, deadline)`.
    async fn connect(
        addr: SocketAddr,
        deadline: Instant,
        connect_timeout: Duration,
        _permit: OwnedSemaphorePermit,
        ttl: Instant,
    ) -> Result<Self, UpstreamError> {
        let effective_deadline = (Instant::now() + connect_timeout).min(deadline);
        let s = timeout_at(effective_deadline, TcpStream::connect(addr))
            .await
            .map_err(|_| UpstreamError::SendTimeout)?
            .map_err(UpstreamError::SendError)?;

        // this allows us to avoid delays in sending small packets, which we are doing in the send_and_receive method.
        s.set_nodelay(true).map_err(UpstreamError::SendError)?;

        const MAX_RECEIVE_BUFFER_SIZE: usize = 65_536;

        Ok(Self {
            stream: s,
            _permit,
            ttl,
            buffer: BytesMut::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
        })
    }

    /// Send a DNS query and receive the response over this TCP connection.
    pub async fn send_and_receive(&mut self, query: &[u8], deadline: Instant) -> Result<Bytes, UpstreamError> {
        if query.len() > u16::MAX as usize {
            return Err(UpstreamError::Other(format!(
                "query too large for DNS/TCP: {}",
                query.len()
            )));
        }

        // write length + body
        // should be fine to write these two separately as they are small and we set tcp_nodelay
        let lenb = (query.len() as u16).to_be_bytes();
        timeout_at(deadline, self.stream.write_all(&lenb))
            .await
            .map_err(|_| UpstreamError::SendTimeout)?
            .map_err(UpstreamError::SendError)?;

        timeout_at(deadline, self.stream.write_all(query))
            .await
            .map_err(|_| UpstreamError::SendTimeout)?
            .map_err(UpstreamError::SendError)?;

        // read resp
        let mut resp_lenb = [0u8; 2];
        timeout_at(deadline, self.stream.read_exact(&mut resp_lenb))
            .await
            .map_err(|_| UpstreamError::RecvTimeout)?
            .map_err(UpstreamError::RecvError)?;
        let n = u16::from_be_bytes(resp_lenb) as usize;

        if self.buffer.capacity() < n {
            self.buffer.reserve(n - self.buffer.capacity());
        }

        self.buffer.resize(n, 0);

        timeout_at(deadline, self.stream.read_exact(&mut self.buffer[..]))
            .await
            .map_err(|_| UpstreamError::RecvTimeout)?
            .map_err(UpstreamError::RecvError)?;

        let resp = self.buffer.split().freeze();
        Ok(resp)
    }
}
