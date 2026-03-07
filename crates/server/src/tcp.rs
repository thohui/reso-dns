use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use arc_swap::ArcSwap;
use bytes::Bytes;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};

use crate::{ServerError, ServerState, handle_request};

/// Run the DNS server over TCP.
#[allow(clippy::too_many_arguments)]
pub async fn run_tcp<G, L>(
    bind_addr: SocketAddr,
    state: Arc<ArcSwap<ServerState<G, L>>>,
    shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()>
where
    L: Default + Send + Sync + 'static,
    G: Send + Sync + 'static,
{
    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("TCP listening on {}", bind_addr);

    // we keep track of the inflight requests so that we can wait for them to finish before shutting down the server.
    let mut inflight = JoinSet::new();

    loop {
        tokio::select! {
            join_res = inflight.join_next(), if !inflight.is_empty() => {
                if let Some(Err(err)) = join_res {
                    tracing::warn!("TCP inflight task failed: {}", err);
                }
            }
            result = listener.accept() => {
                let (mut stream, client) = result?;


                let state = state.clone();

                inflight.spawn(async move {
                    let mut len_buf = [0u8; 2];
                    let mut buf = Vec::new();
                    loop {
                        if let Err(e) = stream.read_exact(&mut len_buf).await {
                            if e.kind() != std::io::ErrorKind::UnexpectedEof {
                                tracing::debug!("failed to read length from client {}: {}", client, e);
                                continue;
                            }
                            return;
                        }

                        let buffer_length = u16::from_be_bytes(len_buf) as usize;
                        buf.resize(buffer_length, 0);

                        if let Err(e) = stream.read_exact(&mut buf).await {
                            if e.kind() != std::io::ErrorKind::UnexpectedEof {
                                tracing::debug!("failed to read length from client {}: {}", client, e);
                                continue;
                            }
                            return;
                        }

                        let bytes = Bytes::copy_from_slice(&buf);
                        let current_state = state.load_full();

                        let mut ctx = DnsRequestCtx::new(
                            current_state.timeout,
                            client,
                            RequestType::TCP,
                            bytes,
                            current_state.global.clone(),
                            L::default(),
                        );

                        match handle_request(&mut ctx, current_state).await {
                            Ok(resp) => {
                                if let Err(e) = write_tcp_response(&mut stream, &resp.bytes()).await {
                                    tracing::debug!("failed to write tcp response to client: {:?}", e);
                                    continue;
                                }
                            }
                            Err(e) => {
                                if let Ok(message) = ctx.message() {
                                    if let Err(e) = write_tcp_server_error_response(message, &mut stream, &e).await {
                                        tracing::debug!("failed to write tcp server response to client: {:?}", e);
                                    }
                                }
                                tracing::debug!("server error: {}", e);
                                continue;
                            }
                        }
                    }
                });
            }
            _ = shutdown.cancelled() => {
                tracing::info!("TCP shutdown signal received, waiting for inflight requests");
                break;
            }
        }
    }

    // wait for in flight requests to finish
    while let Some(join_res) = inflight.join_next().await {
        if let Err(err) = join_res {
            tracing::warn!("TCP inflight task failed during shutdown: {}", err);
        }
    }

    tracing::info!("TCP shutdown complete");

    Ok(())
}

/// Write a DNS friendly response to a TCP stream.
async fn write_tcp_response(stream: &mut tokio::net::TcpStream, response: &Bytes) -> anyhow::Result<()> {
    let len = u16::try_from(response.len()).context("DNS payload exceeds 65535 bytes")?;
    stream.write_u16(len).await?;
    stream.write_all(response).await?;
    Ok(())
}

/// Write a DNS message indicating a server error over TCP.
async fn write_tcp_server_error_response(
    message: &DnsMessage,
    stream: &mut TcpStream,
    error: &ServerError,
) -> anyhow::Result<()> {
    let bytes = DnsMessageBuilder::new()
        .with_id(message.id)
        .with_questions(message.questions().to_vec())
        .with_response(error.response_code())
        .build()
        .encode()?;
    write_tcp_response(stream, &bytes).await?;

    Ok(())
}
