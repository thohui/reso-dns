use std::{net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use doh::run_doh;
use futures::future::BoxFuture;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::DnsResponseCode;
use reso_resolver::{DynResolver, ResolveError, ResolveErrorType};
use tcp::run_tcp;
use udp::run_udp;

use crate::doh::DohConfig;

mod doh;
mod tcp;
mod udp;

pub enum ServerError {
    ResolveError(ResolveError),
    MiddlewareError(anyhow::Error),
}

impl ServerError {
    /// Get the appropriate DNS response code for this error.
    pub fn response_code(&self) -> DnsResponseCode {
        match self {
            ServerError::ResolveError(e) => e.response_code(),
            ServerError::MiddlewareError(_) => DnsResponseCode::ServerFailure,
        }
    }

    /// Get the error type for metrics purposes.
    pub fn error_type(&self) -> ResolveErrorType {
        match self {
            ServerError::ResolveError(e) => e.error_type(),
            ServerError::MiddlewareError(_) => ResolveErrorType::Other,
        }
    }

    /// Get a string representation of the error for logging purposes.
    pub fn to_string(&self) -> String {
        match self {
            ServerError::ResolveError(e) => e.to_string(),
            ServerError::MiddlewareError(e) => e.to_string(),
        }
    }
}

pub type ErrorHandler<G, L> =
    Arc<dyn for<'a> Fn(&'a DnsRequestCtx<G, L>, &'a ServerError) -> BoxFuture<'a, ()> + Send + Sync>;

pub type ServerMiddlewares<G, L> = Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>;

pub struct ServerState<G, L> {
    pub resolver: Arc<DynResolver<G, L>>,
    pub middlewares: ServerMiddlewares<G, L>,
    pub on_error: Option<ErrorHandler<G, L>>,
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
    pub async fn serve_tcp(
        &self,
        bind_addr: SocketAddr,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()> {
        run_tcp(bind_addr, &self.state, shutdown).await
    }

    /// Serve the server over UDP.
    pub async fn serve_udp(
        &self,
        bind_addr: SocketAddr,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()> {
        run_udp(bind_addr, &self.state, shutdown).await
    }

    /// Serve the server over DOH.
    pub async fn serve_doh(&self, bind_addr: SocketAddr, config: DohConfig) -> anyhow::Result<()> {
        run_doh(config, bind_addr, &self.state).await
    }
}

/// Generic request handler that every protocol handler can call into.
pub async fn handle_request<G, L>(
    mut ctx: &mut DnsRequestCtx<G, L>,
    state: Arc<ServerState<G, L>>,
) -> Result<DnsResponse, ServerError>
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    let ServerState {
        resolver, middlewares, ..
    } = &*state;

    for (i, middleware) in state.middlewares.iter().enumerate() {
        if let Some(response) = middleware
            .on_query(&mut ctx)
            .await
            .map_err(ServerError::MiddlewareError)?
        {
            let mut response = response;
            for response_middleware in middlewares.iter().rev() {
                response_middleware
                    .on_response(&mut ctx, &mut response)
                    .await
                    .map_err(ServerError::MiddlewareError)?;
            }
            return Ok(response);
        }
    }

    let mut response = resolver.resolve(&ctx).await.map_err(ServerError::ResolveError)?;

    for middleware in middlewares.iter().rev() {
        middleware
            .on_response(&mut ctx, &mut response)
            .await
            .map_err(ServerError::MiddlewareError)?;
    }

    Ok(response)
}
