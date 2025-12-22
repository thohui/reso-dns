use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use doh::run_doh;
use futures::{FutureExt, future::BoxFuture};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_resolver::{DnsResolver, DynResolver, ResolveError};
use tcp::run_tcp;
use udp::run_udp;

mod doh;
mod tcp;
mod udp;

pub use crate::udp::DohConfig;

pub type SuccessCallback<G, L> =
    Arc<dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a bytes::Bytes) -> BoxFuture<'a, anyhow::Result<()>> + Send + Sync>;

pub type ErrorCallback<G, L> = Arc<
    dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a ResolveError) -> BoxFuture<'a, Result<(), ResolveError>> + Send + Sync,
>;

pub type ServerMiddlewares<G, L> = Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>;

pub struct ServerState<G, L> {
    pub resolver: Arc<DynResolver<G, L>>,
    pub middlewares: ServerMiddlewares<G, L>,
    pub on_success: Option<SuccessCallback<G, L>>,
    pub on_error: Option<ErrorCallback<G, L>>,
    pub global: Arc<G>,
    pub timeout: Duration,
}

/// DNS Server
pub struct DnsServer<G, L> {
    bind_addr: SocketAddr,
    state: ArcSwap<ServerState<G, L>>,
}

impl<L: Default + Send + Sync + 'static, G: Send + Sync + 'static> DnsServer<G, L> {
    pub fn new(bind_addr: SocketAddr, state: ServerState<G, L>) -> Self {
        Self {
            bind_addr,
            state: ArcSwap::new(state.into()),
        }
    }

    pub fn swap_state(&self, new_state: ServerState<G, L>) {
        self.state.swap(new_state.into());
    }

    /// Run the DNS server, listening for incoming requests.
    pub async fn run(self, doh: Option<DohConfig>) -> anyhow::Result<()> {
        let state = &self.state;

        let udp_future = run_udp(self.bind_addr, state).boxed();

        let tcp_future = run_tcp(self.bind_addr, state).boxed();

        let mut futures = vec![udp_future, tcp_future];

        if let Some(doh) = doh {
            let doh_future = run_doh(doh, self.bind_addr, state).boxed();
            futures.push(doh_future);
        }

        futures::future::try_join_all(futures).await?;

        Ok(())
    }
}
