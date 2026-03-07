use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::OwnedSemaphorePermit,
    time::{Duration, Instant, timeout_at},
};

use tokio::sync::Semaphore;

use super::upstream::{Limits, UpstreamError};

/// A pool of TCP connections to a specific upstream server.
/// Existing connections are reused if possible, otherwise new connections are created.
pub(crate) struct TcpPool {
    /// Upstream address
    pub addr: SocketAddr,
    /// Upstream limits
    pub limits: Limits,
    /// Idle connections in insertion order.
    idle: Mutex<VecDeque<TcpConn>>,
    /// Total connections (including in-use and connecting)
    connections: Arc<Semaphore>,
}

impl TcpPool {
    pub fn new(addr: SocketAddr, limits: Limits) -> Arc<Self> {
        Arc::new(Self {
            addr,
            limits,
            idle: Mutex::new(VecDeque::new()),
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
                let mut idle = this.idle.lock().unwrap_or_else(|e| e.into_inner());
                let before = idle.len();
                idle.retain(|c| c.ttl > now);
                let dropped = before - idle.len();
                drop(idle);
                if dropped > 0 {
                    tracing::debug!("reaper dropped {} expired tcp conns to {}", dropped, this.addr);
                }
            }
        });
    }

    /// Try to get an idle conn.
    pub fn try_get(&self) -> Option<TcpConn> {
        // pop from the back to reuse the most recently returned connection, which is likely still alive.
        self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop_back()
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
        if healthy {
            let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
            if idle.len() < self.limits.max_idle_tcp_connections {
                idle.push_back(conn);
            } else {
                tracing::trace!(upstream = %self.addr, "idle pool full, dropping connection");
            }
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
    recv_buf: BytesMut,
    /// Reusable buffer for sending data
    send_buf: Vec<u8>,
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
        // TCP connect can take a long time if the server is unresponsive, so we apply the timeout to the connect operation itself rather than the whole get_or_connect

        let effective_deadline = (Instant::now() + connect_timeout).min(deadline);
        let s = timeout_at(effective_deadline, TcpStream::connect(addr))
            .await
            .map_err(|_| UpstreamError::SendTimeout)?
            .map_err(UpstreamError::SendError)?;

        // this allows us to avoid delays in sending small packets.
        s.set_nodelay(true).map_err(UpstreamError::SendError)?;

        const MAX_RECEIVE_BUFFER_SIZE: usize = 65_536;

        Ok(Self {
            stream: s,
            _permit,
            ttl,
            recv_buf: BytesMut::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
            send_buf: Vec::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
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

        self.send_buf.clear();

        // write length + query.
        self.send_buf.extend_from_slice(&(query.len() as u16).to_be_bytes());
        self.send_buf.extend_from_slice(query);

        timeout_at(deadline, self.stream.write_all(&self.send_buf))
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

        self.recv_buf.reserve(n.saturating_sub(self.recv_buf.capacity()));
        self.recv_buf.resize(n, 0);

        timeout_at(deadline, self.stream.read_exact(&mut self.recv_buf[..]))
            .await
            .map_err(|_| UpstreamError::RecvTimeout)?
            .map_err(UpstreamError::RecvError)?;

        let resp = self.recv_buf.split().freeze();
        Ok(resp)
    }
}
