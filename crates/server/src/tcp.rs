use std::{net::SocketAddr, sync::Arc, time::Duration};

use bytes::Bytes;
use reso_context::{DnsMiddleware, DnsRequestCtx, RequestType};
use reso_dns::{DnsFlags, DnsMessage, DnsResponseCode};
use reso_resolver::DnsResolver;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{ErrorCallback, SuccessCallback};

/// Run the DNS server over TCP.
#[allow(clippy::too_many_arguments)]
pub async fn run_tcp<L, G, R>(
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    timeout: Duration,
    on_success: Option<SuccessCallback<G, L>>,
    on_error: Option<ErrorCallback<G, L>>,
) -> anyhow::Result<()>
where
    L: Default + Send + Sync + 'static,
    G: Send + Sync + 'static,
    R: DnsResolver<G, L> + Send + Sync + 'static,
{
    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("TCP listening on {}", bind_addr);

    loop {
        let (mut stream, client) = listener.accept().await?;

        let resolver = resolver.clone();
        let middlewares = middlewares.clone();
        let global = global.clone();
        let on_success = on_success.clone();
        let on_error = on_error.clone();

        tokio::spawn(async move {
            let mut len_buf = [0u8; 2];
            if let Err(e) = stream.read_exact(&mut len_buf).await {
                tracing::warn!("Failed to read length from client: {} {}", client, e);
                return;
            }

            let buffer_length = u16::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0; buffer_length];
            if let Err(e) = stream.read_exact(&mut buf).await {
                tracing::warn!("Failed to read data from client {}: {}", client, e);
                return;
            }

            let bytes = Bytes::from(buf);

            let ctx = DnsRequestCtx::new(timeout, RequestType::TCP, bytes, global, L::default());

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
                        let res = write_tcp_server_error_response(message, &mut stream).await;
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

/// Write a DNS friendly response to a TCP stream.
async fn write_tcp_response(
    stream: &mut tokio::net::TcpStream,
    response: &Bytes,
) -> anyhow::Result<()> {
    let len = response.len() as u16;

    let mut len_buf = [0u8; 2];
    len_buf.copy_from_slice(&len.to_be_bytes());

    stream.write_all(&len_buf).await?;
    stream.write_all(response).await?;

    Ok(())
}

/// Write a DNS message indicating a server error over TCP.
async fn write_tcp_server_error_response(
    message: &DnsMessage,
    stream: &mut TcpStream,
) -> anyhow::Result<()> {
    let bytes = DnsMessage::new(
        message.id,
        DnsFlags {
            rcode_low: DnsResponseCode::ServerFailure.into(),
            qr: true,
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .encode()?;

    write_tcp_response(stream, &bytes).await?;

    Ok(())
}
