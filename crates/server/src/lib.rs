use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use bytes::BytesMut;
use futures::future::BoxFuture;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_resolver::DnsResolver;
use tokio::{net::UdpSocket, time::timeout};

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
        let socket = Arc::new(UdpSocket::bind(self.bind_addr).await?);
        let mut buffer = BytesMut::with_capacity(self.recv_size);

        tracing::info!("DNS server listening on {}", self.bind_addr);

        loop {
            let sock = socket.clone();

            // TODO: we should not resize the buffer every time, but rather reuse it.
            buffer.resize(self.recv_size, 0);
            let (len, client) = sock.recv_from(&mut buffer[..]).await?;
            let raw = buffer.split_to(len).freeze();

            let resolver = self.resolver.clone();

            let duration = self.timeout;

            let guard = self.middlewares.load();
            let middlewares = guard.clone();
            let global = self.global.clone();

            let on_success = self.on_success.clone();
            let on_error = self.on_error.clone();

            tokio::spawn(async move {
                let ctx = DnsRequestCtx::new(raw, global, L::default());

                if let Ok(Some(resp)) = reso_context::run_middlewares(middlewares, &ctx).await {
                    let _ = sock.send_to(&resp, client).await;

                    if let Some(cb) = &on_success {
                        let _ = cb(&ctx, &resp).await;
                    }
                    return;
                }

                match timeout(duration, resolver.resolve(&ctx)).await {
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
}
