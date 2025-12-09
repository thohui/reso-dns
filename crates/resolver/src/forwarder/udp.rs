use std::net::SocketAddr;

use anyhow::Context;
use bytes::{Bytes, BytesMut};
use reso_dns::helpers;
use tokio::{net::UdpSocket, time::Instant};

/// A single UDP connection to an upstream server.
#[derive(Debug)]
pub(crate) struct UdpConn {
    pub socket: UdpSocket,
}

impl UdpConn {
    /// Create a new UDP connection to the specified upstream address utilizing source port randomization.
    pub async fn new(upstream_addr: SocketAddr) -> anyhow::Result<Self> {
        let bind_addr = if upstream_addr.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from(([0u16; 8], 0))
        };
        let socket = UdpSocket::bind(bind_addr).await?;
        socket.connect(upstream_addr).await?;
        Ok(Self { socket })
    }

    /// Send a DNS query and wait for the response.
    pub async fn send_and_receive(&self, query: &[u8], deadline: Instant) -> anyhow::Result<Bytes> {
        if query.len() > u16::MAX as usize {
            anyhow::bail!("query too large for DNS/UDP: {}", query.len());
        }
        let want_id = u16::from_be_bytes([query[0], query[1]]);

        tokio::time::timeout_at(deadline, self.socket.send(query))
            .await
            .context("send timeout")??;

        const MAX_BUFFER_SIZE: usize = 512;
        let mut buf = BytesMut::with_capacity(MAX_BUFFER_SIZE);
        buf.resize(MAX_BUFFER_SIZE, 0);

        loop {
            let n = tokio::time::timeout_at(deadline, self.socket.recv(&mut buf))
                .await
                .context("recv timeout")??;

            if n >= 12 {
                let got_id = helpers::extract_transaction_id(&buf[..]).unwrap_or_default();
                let qr = (buf[2] & 0x80) != 0;
                if qr && got_id == want_id {
                    buf.truncate(n);
                    return Ok(buf.split().freeze());
                }
            }
        }
    }
}
