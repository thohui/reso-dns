use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use bytes::Bytes;
use dashmap::DashMap;
use reso_dns::helpers;
use tokio::{
    net::UdpSocket,
    sync::{oneshot, watch},
    time::Instant,
};

use crate::forwarder::upstream::UpstreamError;

struct Pending(oneshot::Sender<Bytes>);

/// A multiplexer that sends DNS queries and receives responses over a single
/// UDP socket, correlating them by transaction ID.
pub struct UpstreamUdpMux {
    /// Connected UDP socket to the upstream server.
    socket: Arc<UdpSocket>,
    /// Pending queries keyed by transaction ID. When a response is received.
    pending: Arc<DashMap<u16, Pending>>,
    /// Signals the recv loop to stop when the muxer is dropped.
    _shutdown: watch::Sender<()>,
    /// Set to `false` when the recv loop exits.
    alive: Arc<AtomicBool>,
}

impl UpstreamUdpMux {
    pub async fn new(upstream_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let bind_addr = if upstream_addr.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from(([0u16; 8], 0))
        };

        let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
        socket.connect(upstream_addr).await?;

        let pending = Arc::new(DashMap::<u16, Pending>::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let alive = Arc::new(AtomicBool::new(true));

        {
            let socket = socket.clone();
            let pending = pending.clone();
            let alive = alive.clone();
            tokio::spawn(recv_loop(socket, pending, shutdown_rx, upstream_addr, alive));
        }

        Ok(Self {
            socket,
            pending,
            _shutdown: shutdown_tx,
            alive,
        })
    }

    pub async fn send_and_receive(&self, query: &[u8], deadline: Instant) -> Result<Bytes, UpstreamError> {
        if !self.alive.load(Ordering::Relaxed) {
            return Err(UpstreamError::Other("udp recv loop stopped".into()));
        }

        let query_id = helpers::extract_transaction_id(query)
            .ok_or_else(|| UpstreamError::Other("query too short to contain transaction id".into()))?;

        let (tx, rx) = oneshot::channel();

        // If a pending entry already exists for this transaction ID, the old
        // sender is dropped which causes the old caller to receive a channel
        // closed error rather than silently hanging until timeout.
        self.pending.insert(query_id, Pending(tx));

        match tokio::time::timeout_at(deadline, self.socket.send(query)).await {
            Err(_elapsed) => {
                self.pending.remove(&query_id);
                return Err(UpstreamError::SendTimeout);
            }
            Ok(Err(io_err)) => {
                self.pending.remove(&query_id);
                return Err(UpstreamError::SendError(io_err));
            }
            Ok(Ok(_)) => {}
        }

        match tokio::time::timeout_at(deadline, rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_closed)) => {
                self.pending.remove(&query_id);
                return Err(UpstreamError::RecvTaskStopped);
            }
            Err(_elapsed) => {
                self.pending.remove(&query_id);
                return Err(UpstreamError::RecvTimeout);
            }
        }
    }
}

/// Background task that reads responses from the socket and dispatches them
/// to the corresponding pending callers.
async fn recv_loop(
    socket: Arc<UdpSocket>,
    pending: Arc<DashMap<u16, Pending>>,
    mut shutdown: watch::Receiver<()>,
    upstream_addr: SocketAddr,
    alive: Arc<AtomicBool>,
) {
    const MAX_CONSECUTIVE_ERRORS: u32 = 10;

    // max UDP DNS payload with EDNS is typically 1232-4096 bytes
    // but use a full UDP datagram size buffer to be safe.
    let mut buf = vec![0u8; 65535];
    let mut consecutive_errors: u32 = 0;

    loop {
        let n = tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            result = socket.recv(&mut buf) => {
                match result {
                    Ok(n) => {
                        consecutive_errors = 0;
                        n
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            tracing::error!(
                                upstream = %upstream_addr,
                                error = %e,
                                consecutive_errors,
                                "udp recv loop fatal: too many consecutive errors, exiting"
                            );
                            break;
                        }
                        tracing::warn!(
                            upstream = %upstream_addr,
                            error = %e,
                            consecutive_errors,
                            "udp recv error, continuing"
                        );
                        continue;
                    }
                }
            }
        };

        // a valid DNS header is 12 bytes minimum.
        if n < 12 {
            continue;
        }

        let id = u16::from_be_bytes([buf[0], buf[1]]);

        if let Some((_, Pending(tx))) = pending.remove(&id) {
            let _ = tx.send(Bytes::copy_from_slice(&buf[..n]));
        }
    }

    alive.store(false, Ordering::Relaxed);
}
