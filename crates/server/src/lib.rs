use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use bytes::{Bytes, BytesMut};
use futures::future::BoxFuture;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_resolver::DnsResolver;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
    time::timeout,
};

type SuccessCallback<G, L> = Arc<
    dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a bytes::Bytes) -> BoxFuture<'a, anyhow::Result<()>>
        + Send
        + Sync,
>;

type ErrorCallback<G, L> = Arc<
    dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a anyhow::Error) -> BoxFuture<'a, anyhow::Result<()>>
        + Send
        + Sync,
>;

/// DNS Server
pub struct DnsServer<R, G, L> {
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    recv_size: usize,
    timeout: Duration,
    middlewares: ArcSwap<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    on_success: Option<SuccessCallback<G, L>>,
    on_error: Option<ErrorCallback<G, L>>,
}

impl<
    L: Default + Send + Sync + 'static,
    R: DnsResolver<G, L> + Send + Sync + 'static,
    G: Send + Sync + 'static,
> DnsServer<R, G, L>
{
    pub fn new(bind_addr: SocketAddr, resolver: R, global: Arc<G>) -> Self {
        Self {
            bind_addr,
            resolver: Arc::new(resolver),
            recv_size: 1232, // edns safe
            timeout: Duration::from_secs(2),
            middlewares: ArcSwap::new(Vec::new().into()),
            global,
            on_success: None,
            on_error: None,
        }
    }

    /// Add a handler for successful request processing.
    pub fn add_success_handler(&mut self, handler: SuccessCallback<G, L>) {
        self.on_success = Some(handler);
    }

    /// Add a handler for errors that occur during request processing.
    pub fn add_error_handler(&mut self, handler: ErrorCallback<G, L>) {
        self.on_error = Some(handler);
    }

    /// Add a middleware to the DNS server.
    pub fn add_middleware<M>(&self, mw: M)
    where
        M: DnsMiddleware<G, L> + 'static,
    {
        let cur = self.middlewares.load();
        let mut v = Vec::with_capacity(cur.len() + 1);
        v.extend(cur.iter().cloned());
        v.push(Arc::new(mw));
        self.middlewares.store(Arc::new(v));
    }

    /// Run the DNS server, listening for incoming requests.
    pub async fn run(self) -> anyhow::Result<()> {
        tokio::try_join!(
            run_udp(
                self.bind_addr,
                self.resolver.clone(),
                self.middlewares.load().clone(),
                self.global.clone(),
                self.recv_size,
                self.timeout,
                self.on_success.clone(),
                self.on_error.clone(),
            ),
            run_tcp(
                self.bind_addr,
                self.resolver,
                self.middlewares.load().clone(),
                self.global,
                self.timeout,
                self.on_success,
                self.on_error
            )
        )?;

        Ok(())
    }
}

/// Run the DNS server over UDP.
#[allow(clippy::too_many_arguments)]
async fn run_udp<L, G, R>(
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    recv_size: usize,
    query_timeout: Duration,
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
            let ctx = DnsRequestCtx::new(raw, global, L::default());

            if let Ok(Some(resp)) = reso_context::run_middlewares(middlewares, &ctx).await {
                let _ = sock.send_to(&resp, client).await;

                if let Some(cb) = &on_success {
                    let _ = cb(&ctx, &resp).await;
                }
                return;
            }

            match timeout(query_timeout, resolver.resolve(&ctx)).await {
                Ok(Ok(resp)) => {
                    let _ = sock.send_to(&resp, client).await;

                    if let Some(cb) = &on_success {
                        let _ = cb(&ctx, &resp).await;
                    }
                }
                Ok(Err(e)) => {
                    if let Some(cb) = &on_error {
                        let _ = cb(&ctx, &e).await;
                    }
                }
                Err(err) => {
                    if let Some(cb) = &on_error {
                        let _ = cb(&ctx, &err.into()).await;
                    }
                }
            }
        });
    }
}

/// Run the DNS server over TCP.
#[allow(clippy::too_many_arguments)]
async fn run_tcp<L, G, R>(
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    query_timeout: Duration,
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
            if stream.read_exact(&mut len_buf).await.is_err() {
                tracing::warn!("Failed to read length from client: {}", client);
                return;
            }

            let buffer_length = u16::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0; buffer_length];
            if let Err(e) = stream.read_exact(&mut buf).await {
                tracing::warn!("Failed to read data from client {}: {}", client, e);
                return;
            }

            let bytes = Bytes::from(buf);

            let ctx = DnsRequestCtx::new(bytes, global, L::default());

            if let Ok(Some(resp)) = reso_context::run_middlewares(middlewares, &ctx).await {
                let _ = write_tcp_response(&mut stream, &resp).await;

                if let Some(cb) = &on_success {
                    let _ = cb(&ctx, &resp).await;
                }
                return;
            }

            match timeout(query_timeout, resolver.resolve(&ctx)).await {
                Ok(Ok(resp)) => {
                    let _ = write_tcp_response(&mut stream, &resp).await;

                    if let Some(cb) = &on_success {
                        let _ = cb(&ctx, &resp).await;
                    }
                }
                Ok(Err(e)) => {
                    if let Some(cb) = &on_error {
                        let _ = cb(&ctx, &e).await;
                    }
                }
                Err(err) => {
                    if let Some(cb) = &on_error {
                        let _ = cb(&ctx, &err.into()).await;
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
