use std::{fmt, net::SocketAddr, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use doh::run_doh;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse, ErrorType};
use reso_dns::DnsResponseCode;
use reso_resolver::{DynResolver, ResolveError};
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

    pub fn error_type(&self) -> ErrorType {
        match self {
            ServerError::ResolveError(e) => e.error_type(),
            ServerError::MiddlewareError(_) => ErrorType::Other,
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::ResolveError(e) => write!(f, "{}", e),
            ServerError::MiddlewareError(e) => write!(f, "{}", e),
        }
    }
}

pub type ServerMiddlewares<G, L> = Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>;

pub struct ServerState<G, L> {
    pub resolver: Arc<DynResolver<G, L>>,
    pub middlewares: ServerMiddlewares<G, L>,
    pub global: Arc<G>,
    pub timeout: Duration,
}

/// DNS Server
pub struct DnsServer<G, L> {
    state: Arc<ArcSwap<ServerState<G, L>>>,
}

impl<L: Default + Send + Sync + 'static, G: Send + Sync + 'static> DnsServer<G, L> {
    pub fn new(state: ServerState<G, L>) -> Self {
        Self {
            state: Arc::new(ArcSwap::new(state.into())),
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
        run_tcp(bind_addr, self.state.clone(), shutdown).await
    }

    /// Serve the server over UDP.
    pub async fn serve_udp(
        &self,
        bind_addr: SocketAddr,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()> {
        run_udp(bind_addr, self.state.clone(), shutdown).await
    }

    /// Serve the server over DOH.
    pub async fn serve_doh(&self, bind_addr: SocketAddr, config: DohConfig) -> anyhow::Result<()> {
        run_doh(config, bind_addr, self.state.clone()).await
    }
}

/// Generic request handler that every protocol handler can call into.
pub async fn handle_request<G, L>(
    ctx: &mut DnsRequestCtx<G, L>,
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
        match middleware.on_query(ctx).await {
            Ok(Some(response)) => {
                let mut response = response;
                // Only run on_response for middlewares that had their on_query called.
                for response_middleware in middlewares[..=i].iter().rev() {
                    if let Err(e) = response_middleware
                        .on_response(ctx, &mut response)
                        .await
                        .map_err(ServerError::MiddlewareError)
                    {
                        notify_error(ctx, &middlewares[..=i], &e).await;
                        return Err(e);
                    }
                }
                return Ok(response);
            }
            Ok(None) => {}
            Err(e) => {
                let error = ServerError::MiddlewareError(e);
                notify_error(ctx, &middlewares[..i], &error).await;
                return Err(error);
            }
        }
    }

    match resolver.resolve(ctx).await {
        Ok(mut response) => {
            for middleware in middlewares.iter().rev() {
                middleware
                    .on_response(ctx, &mut response)
                    .await
                    .map_err(ServerError::MiddlewareError)?;
            }
            Ok(response)
        }
        Err(e) => {
            let error = ServerError::ResolveError(e);
            notify_error(ctx, middlewares, &error).await;
            Err(error)
        }
    }
}

/// Notify middlewares that an error occurred, in reverse order.
async fn notify_error<G, L>(
    ctx: &mut DnsRequestCtx<G, L>,
    middlewares: &[Arc<dyn DnsMiddleware<G, L> + 'static>],
    error: &ServerError,
) where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    let error_type = error.error_type();
    let message = error.to_string();
    for middleware in middlewares.iter().rev() {
        middleware.on_error(ctx, &error_type, &message).await;
    }
}
