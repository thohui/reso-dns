use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use aes_gcm::{AesGcm, KeyInit, aead::generic_array::GenericArray};
use anyhow::Context;
use api::serve_web;
use blocklist::service::BlocklistService;
use bytes::Bytes;
use config::{DEFAULT_CONFIG_PATH, ResolverConfig, load_config};
use database::{connect, models::user::User, run_migrations};
use global::{Global, SharedGlobal};
use local::Local;
use metrics::{
    event::{ErrorLogEvent, QueryLogEvent},
    service::MetricsService,
};
use middleware::{blocklist::BlocklistMiddleware, cache::CacheMiddleware};
use moka::future::FutureExt;
use reso_cache::DnsMessageCache;
use reso_context::DnsRequestCtx;
use reso_dns::{DnsMessage, helpers};
use reso_resolver::{ResolveError, forwarder::resolver::ForwardResolver};
use reso_server::{DnsServer, ErrorHandler, ServerMiddlewares, ServerState, SuccessHandler};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utils::password::{generate_password, hash_password};

mod api;
mod blocklist;
mod config;
mod database;
mod global;
mod local;
mod metrics;
mod middleware;
pub mod utils;

#[tokio::main]

async fn main() -> anyhow::Result<()> {
    let (nb, _guard) = non_blocking(std::io::stdout());

    let dns_config_path = env::var("RESO_DNS_CONFIG").unwrap_or(DEFAULT_CONFIG_PATH.to_string());

    let config = load_config(&dns_config_path)?;

    let level_filter = LevelFilter::from(&config.server.log_level);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(nb)
                .with_target(false)
                .with_filter(level_filter),
        )
        .init();

    let connection = Arc::new(connect(&config.database.path).await?);
    run_migrations(&connection).await?;

    let (handle, stats, metrics_service) = MetricsService::new(connection.clone(), 1024);

    let global: SharedGlobal = Arc::new(Global {
        cache: DnsMessageCache::new(50_000),
        blocklist: BlocklistService::new(connection.clone()),
        metrics: handle,
        stats,
        database: connection,
        cipher: AesGcm::new(&GenericArray::clone_from_slice(&config.server.cookie_key)),
        config,
    });

    #[allow(irrefutable_let_patterns)]
    let upstreams = if let ResolverConfig::Forwarder { upstreams } = &global.config.resolver {
        upstreams
    } else {
        return Err(anyhow::anyhow!("Unsupported resolver configuration"));
    };

    let resolver = ForwardResolver::new(&upstreams).await?;

    let timeout_duration = Duration::from_secs(global.config.server.timeout);

    let error_handler: ErrorHandler<Global, Local> =
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
                    qtype = msg.questions().first().and_then(|q| Some(q.qtype as i64));
                }

                let _ = ctx.global().metrics.error(ErrorLogEvent {
                    ts_ms,
                    client: ctx.request_address().to_string(),
                    transport: ctx.request_type(),
                    message: format!("{err}"),
                    r#type: err.error_type(),
                    dur_ms: local.time_elapsed().as_millis() as u64,
                    qname,
                    qtype,
                });

                let id = helpers::extract_transaction_id(&ctx.raw()).unwrap_or(0);
                tracing::error!("error processing request: {}: {}", id, err);

                Ok(())
            }
            .boxed()
        });

    let success_handler: SuccessHandler<Global, Local> =
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
        });

    let middlewares: ServerMiddlewares<Global, Local> =
        Arc::new(vec![Arc::new(BlocklistMiddleware), Arc::new(CacheMiddleware)]);

    let state = ServerState {
        global: global.clone(),
        middlewares,
        on_error: Some(error_handler),
        on_success: Some(success_handler),
        resolver: Arc::new(resolver),
        timeout: timeout_duration,
    };

    let server = DnsServer::<_, Local>::new(state);

    global.blocklist.load_matcher().await?;

    let server_addr = format!("{}:{}", global.config.server.ip, global.config.server.port)
        .parse::<SocketAddr>()
        .expect("invalid server address format");

    let users = User::list(&global.database).await.context("list users")?;

    // Generate admin account if there are no users
    if users.len() == 0 {
        const ADMIN_USERNAME: &str = "admin";
        let password = generate_password(16);
        let password_hash = hash_password(&password)?;
        let admin_user = User::new(ADMIN_USERNAME, password_hash);
        admin_user.insert(&global.database).await.context("create admin user")?;

        tracing::info!(
            "Created a user with username: {} and password: {}",
            ADMIN_USERNAME,
            password
        )
    }

    tokio::select! {
        r = serve_web(global.clone()) => {
            if let Err(e) = r {
                tracing::error!("API exited with error: {}", e);
            }
        }
        r = metrics_service.run() => {
            if let Err(e) = r {
                tracing::error!("Metrics exited with error: {}", e);
            }
        },
        r = server.serve_tcp(server_addr) => {
            if let Err(e) = r {
                tracing::error!("TCP listener exited with error: {}", e);
            }
        },
        r = server.serve_udp(server_addr) => {
            if let Err(e) = r {
                tracing::error!("UDP listener exited with error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down DNS server...");
        },

    }

    Ok(())
}
