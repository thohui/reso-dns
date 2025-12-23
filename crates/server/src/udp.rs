use std::{net::SocketAddr, sync::Arc};

use arc_swap::ArcSwap;
use bytes::BytesMut;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder};
use reso_resolver::ResolveError;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::ServerState;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DohConfig {
    /// Port to listen on for DoH requests.
    pub port: u16,
    /// Path to the TLS certificate file in PEM format.
    pub cert_path: String,
    /// Path to the TLS private key file in PEM format.
    pub key_path: String,
}

/// Run the DNS server over UDP.
#[allow(clippy::too_many_arguments)]
pub async fn run_udp<G, L>(bind_addr: SocketAddr, state: &ArcSwap<ServerState<G, L>>) -> anyhow::Result<()>
where
    L: Default + Send + Sync + 'static,
    G: Send + Sync + 'static,
{
    const RECV_SIZE: usize = 512;

    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    let mut buffer = BytesMut::with_capacity(RECV_SIZE);

    tracing::info!("UDP listening on {}", bind_addr);

    loop {
        let sock = socket.clone();

        let state = state.load_full();

        // TODO: we should not resize the buffer every time, but rather reuse it.
        buffer.resize(RECV_SIZE, 0);
        let (len, client) = sock.recv_from(&mut buffer[..]).await?;
        let raw = buffer.split_to(len).freeze();

        let resolver = state.resolver.clone();

        let middlewares = state.middlewares.clone();
        let global = state.global.clone();

        let on_success = state.on_success.clone();
        let on_error = state.on_error.clone();

        tokio::spawn(async move {
            let ctx = DnsRequestCtx::new(state.timeout, RequestType::UDP, raw, global, L::default());

            if let Ok(Some(resp)) = reso_context::run_middlewares(middlewares, &ctx).await {
                let _ = sock.send_to(&resp, client).await;

                if let Some(cb) = &on_success {
                    let _ = cb(&ctx, &resp).await;
                }
                return;
            }

            match resolver.resolve(&ctx).await {
                Ok(resp) => {
                    let _ = sock.send_to(&resp, client).await;

                    if let Some(cb) = &on_success {
                        let _ = cb(&ctx, &resp).await;
                    }
                }
                Err(e) => {
                    if let Ok(message) = ctx.message() {
                        let res = write_udp_server_error_response(message, &sock, &client, &e).await;
                        if let Err(err) = res {
                            tracing::warn!("Failed to write error response to client {}: {}", client, err);
                        }
                    }
                    if let Some(cb) = &on_error {
                        let _ = cb(&ctx, &e).await;
                    }
                }
            }
        });
    }
}

/// Write a DNS message indicating a server error over UDP.
async fn write_udp_server_error_response(
    message: &DnsMessage,
    socket: &UdpSocket,
    client: &SocketAddr,
    error: &ResolveError,
) -> anyhow::Result<()> {
    let bytes = DnsMessageBuilder::new()
        .with_id(message.id)
        .with_questions(message.questions().to_vec())
        .with_response(error.response_code())
        .build()
        .encode()?;

    socket.send_to(&bytes, client).await?;

    Ok(())
}
