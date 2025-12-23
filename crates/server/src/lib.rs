use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use doh::run_doh;
use futures::future::BoxFuture;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_resolver::{DynResolver, ResolveError};
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
    state: ArcSwap<ServerState<G, L>>,
}

impl<L: Default + Send + Sync + 'static, G: Send + Sync + 'static> DnsServer<G, L> {
    pub fn new(state: ServerState<G, L>) -> Self {
        Self {
            state: ArcSwap::new(state.into()),
        }
    }

    pub fn swap_state(&self, new_state: ServerState<G, L>) {
        self.state.swap(new_state.into());
    }

    /// Serve the server over TCP.
    pub async fn serve_tcp(&self, bind_addr: SocketAddr) -> anyhow::Result<()> {
        run_tcp(bind_addr, &self.state).await
    }

    /// Serve the server over UDP.
    pub async fn serve_udp(&self, bind_addr: SocketAddr) -> anyhow::Result<()> {
        run_udp(bind_addr, &self.state).await
    }

    /// Serve the server over DOH.
    pub async fn serve_doh(&self, bind_addr: SocketAddr, config: DohConfig) -> anyhow::Result<()> {
        run_doh(config, bind_addr, &self.state).await
    }
}
