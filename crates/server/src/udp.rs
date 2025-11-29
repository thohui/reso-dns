use std::{net::SocketAddr, sync::Arc, time::Duration};

use bytes::BytesMut;
use reso_context::{DnsMiddleware, DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder, DnsResponseCode};
use reso_resolver::{DnsResolver, ResolveError};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::{ErrorCallback, SuccessCallback};

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
pub async fn run_udp<L, G, R>(
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    recv_size: usize,
    timeout: Duration,
    on_success: Option<SuccessCallback<G, L>>,
    on_error: Option<ErrorCallback<G, L>>,
) -> anyhow::Result<()>
where
    L: Default + Send + Sync + 'static,
    G: Send + Sync + 'static,
    R: DnsResolver<G, L> + Send + Sync + 'static,
{
    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    let mut buffer = BytesMut::with_capacity(recv_size);

    tracing::info!("UDP listening on {}", bind_addr);

    loop {
        let sock = socket.clone();

        // TODO: we should not resize the buffer every time, but rather reuse it.
        buffer.resize(recv_size, 0);
        let (len, client) = sock.recv_from(&mut buffer[..]).await?;
        let raw = buffer.split_to(len).freeze();

        let resolver = resolver.clone();

        let middlewares = middlewares.clone();
        let global = global.clone();

        let on_success = on_success.clone();
        let on_error = on_error.clone();

        tokio::spawn(async move {
            let ctx = DnsRequestCtx::new(timeout, RequestType::UDP, raw, global, L::default());

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
                        let res =
                            write_udp_server_error_response(message, &sock, &client, &e).await;
                        if let Err(err) = res {
                            tracing::warn!(
                                "Failed to write error response to client {}: {}",
                                client,
                                err
                            );
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
