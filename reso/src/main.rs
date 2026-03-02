use std::{sync::Arc, time::Duration};

use aes_gcm::{AesGcm, KeyInit, aead::generic_array::GenericArray};
use api::serve_web;
use database::{connect, run_migrations};
use env_config::EnvConfig;
use global::{Global, SharedGlobal};
use metrics::service::MetricsService;
use reso_cache::DnsMessageCache;
use server_builder::{build_dns_server, update_server_state_on_config_changes};
use services::{blocklist::BlocklistService, config::ConfigService};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod database;
mod env_config;
mod global;
mod local;
mod metrics;
mod middleware;
mod server_builder;
mod services;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (nb, _guard) = non_blocking(std::io::stdout());

    let config = EnvConfig::from_env()?;

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(nb)
                .with_target(false)
                .with_filter(LevelFilter::from(config.log_level)),
        )
        .init();

    let connection = Arc::new(connect(&config.db_path).await?);
    run_migrations(&connection).await?;

    let (handle, stats, metrics_service) = MetricsService::new(connection.clone(), 1000);

    let global: SharedGlobal = Arc::new(Global {
        cache: DnsMessageCache::new(50_000),
        blocklist: BlocklistService::initialize(connection.clone()).await?,
        config_service: ConfigService::initialize(connection.clone()).await?,
        metrics: handle,
        stats,
        database: connection,
        cipher: AesGcm::new(&GenericArray::clone_from_slice(&config.cookie_secret)),
    });

    let server = build_dns_server(global.clone()).await?;

    let global_clone = global.clone();

    let shutdown = tokio_util::sync::CancellationToken::new();

    let dns_udp_shutdown = shutdown.child_token();
    let dns_tcp_shutdown = shutdown.child_token();
    let metrics_shutdown = shutdown.child_token();
    let web_shutdown = shutdown.child_token();

    let udp_clone = server.clone();
    let tcp_clone = server.clone();

    let dns_udp_handle =
        tokio::spawn(async move { udp_clone.serve_udp(config.dns_server_address, dns_udp_shutdown).await });
    let dns_tcp_handle =
        tokio::spawn(async move { tcp_clone.serve_tcp(config.dns_server_address, dns_tcp_shutdown).await });

    let metrics_handle = tokio::spawn(metrics_service.run(metrics_shutdown.clone()));
    let web_handle = tokio::spawn(serve_web(config.http_server_address, global, web_shutdown.clone()));

    let _ = tokio::spawn(async move { update_server_state_on_config_changes(global_clone, server).await });

    signal::ctrl_c().await?;
    tracing::info!("shutdown signal received");

    shutdown.cancel();

    let drain = async {
        let _ = dns_udp_handle.await;
        let _ = dns_tcp_handle.await;
        let _ = web_handle.await;
    };

    match tokio::time::timeout(Duration::from_secs(10), drain).await {
        Ok(_) => tracing::info!("all connections drained"),
        Err(_) => tracing::warn!("drain timeout, forcing shutdown"),
    }

    tracing::info!("waiting for metrics service to shut down");
    let _ = metrics_handle.await;
    tracing::info!("metrics service shut down");

    tracing::info!("shutdown complete");

    Ok(())
}
