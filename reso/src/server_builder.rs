use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use futures::{FutureExt, StreamExt};
use reso_context::DnsRequestCtx;
use reso_dns::{DnsMessage, helpers};
use reso_resolver::{ResolveError, forwarder::resolver::ForwardResolver};
use reso_server::{DnsServer, ErrorHandler, ServerMiddlewares, ServerState, SuccessHandler};
use tokio_stream::wrappers::WatchStream;

use crate::{
    global::{Global, SharedGlobal},
    local::Local,
    metrics::event::{ErrorLogEvent, QueryLogEvent},
    middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware},
    services::{self, config::model::ActiveResolver},
};

pub fn success_handler() -> SuccessHandler<Global, Local> {
    Arc::new(|ctx: &DnsRequestCtx<Global, Local>, resp: &Bytes| {
        async move {
            let message = ctx.message()?;

            if !ctx.local().cache_hit {
                let resp_msg = DnsMessage::decode(resp)?;
                let _ = ctx.global().cache.insert(message, &resp_msg).await;
            }

            let local = ctx.local();

            let ts_ms: i64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as i64;

            // This should be safe as the questions are validated earlier on in the resolver.
            let question = message.questions().first().unwrap();

            let response = DnsMessage::decode(&resp)?;

            ctx.global().metrics.query(QueryLogEvent {
                ts_ms,
                transport: ctx.request_type(),
                client: ctx.request_address().to_string(),
                qname: question.qname.clone(),
                qtype: question.qtype,
                rcode: response.response_code()?,
                dur_ms: local.time_elapsed().as_millis() as u64,
                cache_hit: local.cache_hit,
                blocked: local.blocked,
            });

            Ok(())
        }
        .boxed()
    })
}

pub fn error_handler() -> ErrorHandler<Global, Local> {
    Arc::new(|ctx: &DnsRequestCtx<Global, Local>, err: &ResolveError| {
        async move {
            let local = ctx.local();
            let ts_ms: i64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("failed to get the system time")
                .as_millis() as i64;

            let mut qname = None;
            let mut qtype = None;

            // try to get qname and qtype
            if let Ok(msg) = ctx.message() {
                qname = msg.questions().first().and_then(|q| Some(q.qname.to_string()));
                qtype = msg.questions().first().and_then(|q| Some(q.qtype.to_u16() as i64));
            }

            let _ = ctx.global().metrics.error(ErrorLogEvent {
                ts_ms,
                client: ctx.request_address().to_string(),
                transport: ctx.request_type(),
                message: err.to_string(),
                r#type: err.error_type(),
                dur_ms: local.time_elapsed().as_millis() as u64,
                qname,
                qtype,
            });

            let id = helpers::extract_transaction_id(&ctx.raw()).unwrap_or(0);
            tracing::debug!("error processing request: {}: {}", id, err);

            Ok(())
        }
        .boxed()
    })
}

pub fn server_middlewares() -> ServerMiddlewares<Global, Local> {
    let middlewares: ServerMiddlewares<Global, Local> =
        Arc::new(vec![Arc::new(BlocklistMiddleware), Arc::new(CacheMiddleware)]);
    middlewares
}

async fn create_server_state(
    global: &SharedGlobal,
    config: &services::config::model::Config,
) -> anyhow::Result<ServerState<Global, Local>> {
    let resolver = match &config.dns.active {
        ActiveResolver::Forwarder => ForwardResolver::new(&config.dns.forwarder.upstreams).await?,
    };

    Ok(ServerState {
        timeout: Duration::from_secs(config.dns.timeout),
        global: global.clone(),
        middlewares: server_middlewares(),
        on_error: Some(error_handler()),
        on_success: Some(success_handler()),
        resolver: Arc::new(resolver),
    })
}

pub async fn update_server_state_on_config_changes(global: SharedGlobal, server: Arc<DnsServer<Global, Local>>) {
    let mut rx = global.config_service.subscribe();

    // skip the initial value.
    rx.mark_unchanged();

    let mut stream = WatchStream::from_changes(rx);
    while let Some(updated_config) = stream.next().await {
        tracing::info!("applying new configuration changes");
        match create_server_state(&global, &updated_config).await {
            Ok(state) => {
                server.swap_state(state);
            }
            Err(e) => {
                tracing::error!("failed to update dns server with new config: {}", e);
            }
        }
    }
}

pub async fn create_dns_server(global: SharedGlobal) -> anyhow::Result<Arc<DnsServer<Global, Local>>> {
    let config = global.config_service.get_config();
    let server_state = create_server_state(&global, &config).await?;
    Ok(Arc::new(DnsServer::new(server_state)))
}
