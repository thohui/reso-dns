use std::{sync::Arc, time::Duration};
use tokio::runtime::Builder;

use aes_gcm::{AesGcm, KeyInit, aead::generic_array::GenericArray};
use api::serve_web;
use database::{connect_core_db, run_core_db_migrations};
use env_config::EnvConfig;
use global::{Global, SharedGlobal};
use metrics::{service::MetricsService, truncation::run_metrics_truncation};
use reso_cache::DnsMessageCache;
use server_builder::{build_dns_server, update_server_state_on_config_changes};
use services::{blocklist::BlocklistService, config::ConfigService};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    database::{connect_metrics_db, run_metrics_db_migrations},
    services::local_records::LocalRecordService,
};

mod api;
mod database;
mod env_config;
mod global;
mod local;
mod metrics;
mod middleware;
mod ratelimit;
mod server_builder;
mod services;
mod utils;

fn main() -> anyhow::Result<()> {
    let worker_threads = std::thread::available_parallelism()?.get();
    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()?;
    runtime.block_on(run())
}

async fn run() -> anyhow::Result<()> {
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

    let core_db_connection = Arc::new(connect_core_db(&config.db_path).await?);
    run_core_db_migrations(&core_db_connection).await?;

    let metrics_db_connection = Arc::new(connect_metrics_db(&config.metrics_db_path).await?);
    run_metrics_db_migrations(&metrics_db_connection).await?;

    let (handle, stats, metrics_service) = MetricsService::new(metrics_db_connection.clone(), 1000);

    let global: SharedGlobal = Arc::new(Global {
        cache: DnsMessageCache::new(50_000),
        blocklist: BlocklistService::initialize(core_db_connection.clone()).await?,
        local_records_service: LocalRecordService::initialize(core_db_connection.clone()).await?,
        config_service: ConfigService::initialize(core_db_connection.clone()).await?,
        metrics: handle,
        stats,
        core_database: core_db_connection,
        metrics_database: metrics_db_connection.clone(),
        cipher: AesGcm::new(&GenericArray::clone_from_slice(&config.cookie_secret)),
    });

    let server = build_dns_server(global.clone()).await?;

    let shutdown = tokio_util::sync::CancellationToken::new();

    let dns_udp_shutdown = shutdown.child_token();
    let dns_tcp_shutdown = shutdown.child_token();
    let metrics_shutdown = shutdown.child_token();
    let web_shutdown = shutdown.child_token();

    let udp_clone = server.clone();
    let tcp_clone = server.clone();

    let dns_udp_handle = tokio::spawn(async move {
        if let Err(e) = udp_clone.serve_udp(config.dns_server_address, dns_udp_shutdown).await {
            tracing::error!("UDP server failed: {}", e);
        }
    });
    let dns_tcp_handle = tokio::spawn(async move {
        if let Err(e) = tcp_clone.serve_tcp(config.dns_server_address, dns_tcp_shutdown).await {
            tracing::error!("TCP server failed: {}", e);
        }
    });

    let metrics_handle = tokio::spawn(metrics_service.run(metrics_shutdown.clone()));

    let truncate_shutdown = shutdown.child_token();
    let truncate_db = metrics_db_connection.clone();
    let truncate_config_rx = global.config_service.subscribe();
    let truncate_handle = tokio::spawn(run_metrics_truncation(
        truncate_db,
        truncate_config_rx,
        truncate_shutdown,
    ));
    let web_handle = tokio::spawn(serve_web(
        config.http_server_address,
        global.clone(),
        web_shutdown.clone(),
    ));

    let task_global = global.clone();
    let _ = tokio::spawn(async move { update_server_state_on_config_changes(task_global, server).await });

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        signal::ctrl_c().await?;
    }

    tracing::info!("shutdown signal received");
    shutdown.cancel();

    let drain = async {
        let _ = dns_udp_handle.await;
        let _ = dns_tcp_handle.await;
        let _ = web_handle.await;
        let _ = truncate_handle.await;
    };

    match tokio::time::timeout(Duration::from_secs(10), drain).await {
        Ok(_) => tracing::info!("all connections drained"),
        Err(_) => tracing::warn!("drain timeout, forcing shutdown"),
    }

    if let Err(e) = &global
        .metrics_database
        .interact(|c| c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);"))
        .await
    {
        tracing::error!("failed to checkpoint metrics database: {}", e);
    };

    if let Err(e) = &global
        .core_database
        .interact(|c| c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);"))
        .await
    {
        tracing::error!("failed to checkpoint core database: {}", e);
    }

    tracing::info!("waiting for metrics service to shut down");
    let _ = metrics_handle.await;
    tracing::info!("metrics service shut down");

    tracing::info!("shutdown complete");

    Ok(())
}
