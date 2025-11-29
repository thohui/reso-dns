use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use doh::run_doh;
use futures::{FutureExt, future::BoxFuture};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_resolver::{DnsResolver, ResolveError};
use tcp::run_tcp;
use udp::run_udp;

mod doh;
mod tcp;
mod udp;

pub use crate::udp::DohConfig;

type SuccessCallback<G, L> = Arc<
    dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a bytes::Bytes) -> BoxFuture<'a, anyhow::Result<()>>
        + Send
        + Sync,
>;

type ErrorCallback<G, L> = Arc<
    dyn for<'a> Fn(
            &'a DnsRequestCtx<G, L>,
            &'a ResolveError,
        ) -> BoxFuture<'a, Result<(), ResolveError>>
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
    pub fn new(bind_addr: SocketAddr, resolver: R, timeout: Duration, global: Arc<G>) -> Self {
        Self {
            bind_addr,
            resolver: Arc::new(resolver),
            recv_size: 1232, // edns safe
            timeout,
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
    pub async fn run(self, doh: Option<DohConfig>) -> anyhow::Result<()> {
        let udp_future = run_udp(
            self.bind_addr,
            self.resolver.clone(),
            self.middlewares.load().clone(),
            self.global.clone(),
            self.recv_size,
            self.timeout,
            self.on_success.clone(),
            self.on_error.clone(),
        )
        .boxed();

        let tcp_future = run_tcp(
            self.bind_addr,
            self.resolver.clone(),
            self.middlewares.load().clone(),
            self.global.clone(),
            self.timeout,
            self.on_success.clone(),
            self.on_error.clone(),
        )
        .boxed();

        let mut futures = vec![udp_future, tcp_future];

        if let Some(doh) = doh {
            let doh_future = run_doh(
                doh,
                self.bind_addr,
                self.resolver,
                self.middlewares.load().clone(),
                self.global,
                self.recv_size,
                self.timeout,
                self.on_success,
                self.on_error,
            )
            .boxed();
            futures.push(doh_future);
        }

        futures::future::try_join_all(futures).await?;

        Ok(())
    }
}
