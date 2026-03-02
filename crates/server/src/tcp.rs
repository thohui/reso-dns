use std::net::SocketAddr;

use anyhow::Context;
use arc_swap::ArcSwap;
use bytes::Bytes;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder};
use reso_resolver::ResolveError;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};

use crate::ServerState;

/// Run the DNS server over TCP.
#[allow(clippy::too_many_arguments)]
pub async fn run_tcp<G, L>(
    bind_addr: SocketAddr,
    state: &ArcSwap<ServerState<G, L>>,
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

                let state = state.load_full();
                let resolver = state.resolver.clone();
                let middlewares = state.middlewares.clone();
                let global = state.global.clone();
                let on_success = state.on_success.clone();
                let on_error = state.on_error.clone();

                inflight.spawn(async move {
                    let mut len_buf = [0u8; 2];
                    if let Err(e) = stream.read_exact(&mut len_buf).await {
                        tracing::debug!("failed to read length from client: {} {}", client, e);
                        return;
                    }

                    let buffer_length = u16::from_be_bytes(len_buf) as usize;

                    let mut buf = vec![0; buffer_length];

                    if let Err(e) = stream.read_exact(&mut buf).await {
                        tracing::debug!("failed to read data from client {}: {}", client, e);
                        return;
                    }

                    let bytes = Bytes::from(buf);

                    let ctx = DnsRequestCtx::new(
                        state.timeout,
                        client.into(),
                        RequestType::TCP,
                        bytes,
                        global,
                        L::default(),
                    );

                    if let Ok(Some(resp)) = reso_context::run_middlewares(middlewares, &ctx).await {
                        let _ = write_tcp_response(&mut stream, &resp).await;

                        if let Some(cb) = &on_success {
                            let _ = cb(&ctx, &resp).await;
                        }
                        return;
                    }

                    match resolver.resolve(&ctx).await {
                        Ok(resp) => {
                            let _ = write_tcp_response(&mut stream, &resp).await;

                            if let Some(cb) = &on_success {
                                let _ = cb(&ctx, &resp).await;
                            }
                        }
                        Err(e) => {
                            if let Ok(message) = ctx.message() {
                                let res = write_tcp_server_error_response(message, &mut stream, &e).await;
                                if let Err(err) = res {
                                    tracing::warn!("failed to write error response to client {}: {}", client, err);
                                }
                            }
                            if let Some(cb) = &on_error {
                                let _ = cb(&ctx, &e).await;
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
    error: &ResolveError,
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
