use std::{net::SocketAddr, sync::Arc};

use arc_swap::ArcSwap;
use bytes::Bytes;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder};
use tokio::{net::UdpSocket, task::JoinSet};

use crate::{ServerError, ServerState, handle_request};

/// Run the DNS server over UDP.
pub async fn run_udp<G, L>(
    bind_addr: SocketAddr,
    state: Arc<ArcSwap<ServerState<G, L>>>,
    shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()>
where
    L: Default + Send + Sync + 'static,
    G: Send + Sync + 'static,
{
    const RECV_SIZE: usize = 1232;

    let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
    let mut buffer = vec![0; RECV_SIZE];

    tracing::info!("UDP listening on {}", bind_addr);

    // we keep track of the inflight requests so that we can wait for them to finish before shutting down the server.
    let mut inflight = JoinSet::new();

    loop {
        tokio::select! {
            join_res = inflight.join_next(), if !inflight.is_empty() => {
                if let Some(Err(err)) = join_res {
                    tracing::warn!("UDP inflight task failed: {}", err);
                }
            }
            result = socket.recv_from(&mut buffer[..]) => {
                let (len, client) = result?;
                let raw = Bytes::copy_from_slice(&buffer[..len]);
                let sock = socket.clone();

                let state = state.load_full();
                let global = state.global.clone();

                inflight.spawn(async move {
                    let mut ctx = DnsRequestCtx::new(state.timeout, client.ip(), RequestType::UDP, raw, global, L::default());

                    match handle_request(&mut ctx, state).await {
                        Ok(resp) => {
                            let _ = sock.send_to(&resp.bytes(), client).await;
                        },
                        Err(e) => {
                            if let Ok(message) = ctx.message() {
                                let res = write_udp_server_error_response(message, &sock, &client, &e).await;
                                if let Err(err) = res {
                                    tracing::warn!("failed to write error response to client {}: {}", client, err);
                                }
                            }
                        }
                    }
                });
            }
            _ = shutdown.cancelled() => {
                tracing::info!("UDP shutdown signal received, waiting for inflight requests");
                break;
            }
        }
    }

    // wait for in flight requests to finish
    while let Some(join_res) = inflight.join_next().await {
        if let Err(err) = join_res {
            tracing::warn!("UDP inflight task failed during shutdown: {}", err);
        }
    }

    tracing::info!("UDP shutdown complete");

    Ok(())
}

/// Write a DNS message indicating a server error over UDP.
async fn write_udp_server_error_response(
    message: &DnsMessage,
    socket: &UdpSocket,
    client: &SocketAddr,
    error: &ServerError,
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
