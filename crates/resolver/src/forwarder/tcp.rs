use std::{
    collections::VecDeque,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpStream,
    sync::OwnedSemaphorePermit,
    time::{Duration, Instant, timeout_at},
};
use tokio_rustls::client::TlsStream;

use tokio::sync::{Notify, Semaphore};

use super::{
    dot::TlsUpstream,
    upstream::{Limits, UpstreamError},
};

enum Stream {
    Plain(TcpStream),
    // boxed to decrease the overall footprint of the Stream enum.
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Stream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_read(cx, buf),
            Self::Tls(s) => Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_write(cx, buf),
            Self::Tls(s) => Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_flush(cx),
            Self::Tls(s) => Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_shutdown(cx),
            Self::Tls(s) => Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

/// A pool of TCP connections to a specific upstream server.
/// Existing connections are reused if possible, otherwise new connections are created.
pub(crate) struct TcpPool {
    /// Upstream address
    pub addr: SocketAddr,
    /// Upstream limits
    pub limits: Limits,
    /// TLS parameters when this upstream is DoT, None for plain DNS over TCP.
    tls: Option<Arc<TlsUpstream>>,
    /// Idle connections in insertion order.
    idle: Mutex<VecDeque<TcpConn>>,
    /// Total connections
    connections: Arc<Semaphore>,
    /// Woken when a connection is put back into the idle pool.
    returned: Notify,
}

impl TcpPool {
    pub fn new(addr: SocketAddr, limits: Limits, tls: Option<Arc<TlsUpstream>>) -> Arc<Self> {
        Arc::new(Self {
            addr,
            limits,
            tls,
            idle: Mutex::new(VecDeque::new()),
            connections: Arc::new(Semaphore::new(limits.max_tcp_connections)),
            returned: Notify::new(),
        })
    }

    /// Start a background task that reaps expired idle tcp connections.
    pub fn start_reaper(self: Arc<Self>, interval: Duration) {
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
        let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        while let Some(mut conn) = idle.pop_back() {
            if conn.ttl > now && conn.is_alive() {
                return Some(conn);
            }
            tracing::debug!(upstream = %self.addr, "discarding closed idle tcp connection");
        }
        None
    }

    /// Get an idle conn or connect a new one, waiting for a free slot when at the cap.
    pub async fn get_or_connect(&self, deadline: Instant) -> Result<(TcpConn, bool), UpstreamError> {
        // capped so a saturated pool doesn't eat the budget needed to fail over to another upstream
        let wait_deadline = (Instant::now() + self.limits.connect_timeout).min(deadline);

        let permit = match timeout_at(wait_deadline, self.wait_for_slot()).await {
            Ok(Ok(Slot::Idle(conn))) => return Ok((conn, true)),
            Ok(Ok(Slot::New(permit))) => permit,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(UpstreamError::PoolExhausted),
        };

        Ok((self.connect(permit, deadline).await?, false))
    }

    async fn wait_for_slot(&self) -> Result<Slot, UpstreamError> {
        loop {
            // idle connections keep their permit, so also wake up when one is put back.
            // registered before try_get so a put_back in between isn't missed
            let returned = self.returned.notified();
            tokio::pin!(returned);
            returned.as_mut().enable();

            if let Some(conn) = self.try_get() {
                tracing::debug!(upstream = %self.addr, "reusing idle tcp connection");
                return Ok(Slot::Idle(conn));
            }

            tokio::select! {
                permit = self.acquire_permit() => return Ok(Slot::New(permit?)),
                _ = &mut returned => {}
            }
        }
    }

    async fn acquire_permit(&self) -> Result<OwnedSemaphorePermit, UpstreamError> {
        self.connections
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| UpstreamError::Other(format!("upstream {} connection pool closed", self.addr)))
    }

    /// Open a new connection in the slot held by `permit`.
    pub async fn connect(&self, permit: OwnedSemaphorePermit, deadline: Instant) -> Result<TcpConn, UpstreamError> {
        tracing::debug!(upstream = %self.addr, tls = self.tls.is_some(), "opening new tcp connection");

        TcpConn::connect(
            self.addr,
            self.tls.as_deref(),
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
                drop(idle);
                self.returned.notify_one();
            } else {
                tracing::trace!(upstream = %self.addr, "idle pool full, dropping connection");
            }
        }
    }

    pub fn has_tls(&self) -> bool {
        self.tls.is_some()
    }
}

enum Slot {
    Idle(TcpConn),
    New(OwnedSemaphorePermit),
}

/// A single connection to an upstream server, plain or TLS.
pub struct TcpConn {
    /// The transport
    stream: Stream,
    /// Permit that keeps the connection slot
    permit: OwnedSemaphorePermit,
    /// Time-to-live for this connection
    pub ttl: Instant,
    /// Reusable buffer for receiving data
    recv_buf: BytesMut,
    /// Reusable buffer for sending data
    send_buf: Vec<u8>,
}

impl TcpConn {
    /// Establish a new connection to the given address with a timeout and a permit.
    /// The effective timeout is `min(now + connect_timeout, deadline)`.
    async fn connect(
        addr: SocketAddr,
        tls: Option<&TlsUpstream>,
        deadline: Instant,
        connect_timeout: Duration,
        permit: OwnedSemaphorePermit,
        ttl: Instant,
    ) -> Result<Self, UpstreamError> {
        // TCP connect can take a long time if the server is unresponsive, so we apply the
        // timeout to the connect operation itself rather than the whole get_or_connect.
        // The TLS handshake sits inside the same budget: it is a second round trip an
        // unresponsive server could otherwise stall on indefinitely.
        let effective_deadline = (Instant::now() + connect_timeout).min(deadline);

        let stream = timeout_at(effective_deadline, Self::open(addr, tls))
            .await
            .map_err(|_| UpstreamError::SendTimeout)??;

        const MAX_RECEIVE_BUFFER_SIZE: usize = 65_536;

        Ok(Self {
            stream,
            permit,
            ttl,
            recv_buf: BytesMut::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
            send_buf: Vec::with_capacity(MAX_RECEIVE_BUFFER_SIZE),
        })
    }

    async fn open(addr: SocketAddr, tls: Option<&TlsUpstream>) -> Result<Stream, UpstreamError> {
        let sock = TcpStream::connect(addr).await.map_err(UpstreamError::SendError)?;

        // this allows us to avoid delays in sending small packets.
        sock.set_nodelay(true).map_err(UpstreamError::SendError)?;

        match tls {
            Some(tls) => Ok(Stream::Tls(Box::new(tls.handshake(sock).await?))),
            None => Ok(Stream::Plain(sock)),
        }
    }

    /// Close the connection but keep its slot, so a replacement doesn't have to queue for one.
    pub fn into_permit(self) -> OwnedSemaphorePermit {
        self.permit
    }

    /// Check if the connection is still open without blocking.
    /// In some cases the server has already closed the connection when a tcp conn is reused from the pool.
    fn is_alive(&mut self) -> bool {
        let mut buf = [0u8; 1];
        let mut read_buf = ReadBuf::new(&mut buf);
        let mut cx = Context::from_waker(Waker::noop());

        // Pending = nothing to read aka idle and still open.
        // Ready = EOF, error etc

        match &mut self.stream {
            Stream::Plain(stream) => stream.poll_peek(&mut cx, &mut read_buf).is_pending(),
            Stream::Tls(s) => Pin::new(s.as_mut()).poll_read(&mut cx, &mut read_buf).is_pending(),
        }
    }

    /// Send a DNS query and receive the response over this connection.
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

        if n < 12 {
            return Err(UpstreamError::RecvError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("upstream response length {n} is below minimum DNS message size"),
            )));
        }

        self.recv_buf.resize(n, 0);

        timeout_at(deadline, self.stream.read_exact(&mut self.recv_buf[..]))
            .await
            .map_err(|_| UpstreamError::RecvTimeout)?
            .map_err(UpstreamError::RecvError)?;

        let resp = self.recv_buf.split().freeze();
        Ok(resp)
    }
}
