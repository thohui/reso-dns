use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::Context;
use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::OwnedSemaphorePermit,
    time::{Duration, Instant, timeout, timeout_at},
};

use crossbeam_queue::SegQueue;
use tokio::sync::Semaphore;

use super::upstream::Limits;

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
            _ = tokio::time::sleep_until(deadline) => Err(anyhow::anyhow!("deadline reached")),
            res = self.get_or_connect_inner() => res
        }
    }

    async fn get_or_connect_inner(&self) -> anyhow::Result<TcpConn> {
        if let Some(c) = self.try_get() {
            return Ok(c);
        }

        let permit = self.connections.clone().try_acquire_owned().map_err(|_| {
            anyhow::anyhow!(
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
        if healthy && self.idle_count.load(Ordering::Relaxed) < self.limits.max_idle_tcp_connections
        {
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

        const MAX_RECEIVE_BUFFER_SIZE: usize = 65_536;

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
