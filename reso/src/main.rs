use std::sync::Arc;

use aes_gcm::{AesGcm, KeyInit, aead::generic_array::GenericArray};
use anyhow::Context;
use api::serve_web;
use database::{connect, models::user::User, run_migrations};
use env_config::EnvConfig;
use global::{Global, SharedGlobal};
use metrics::service::MetricsService;
use reso_cache::DnsMessageCache;
use server_builder::{create_dns_server, update_server_state_on_config_changes};
use services::{blocklist::BlocklistService, config::ConfigService};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utils::password::{generate_password, hash_password};

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

    let (handle, stats, metrics_service) = MetricsService::new(connection.clone(), 1024);

    let global: SharedGlobal = Arc::new(Global {
        cache: DnsMessageCache::new(50_000),
        blocklist: BlocklistService::initialize(connection.clone()).await?,
        config_service: ConfigService::initialize(connection.clone()).await?,
        metrics: handle,
        stats,
        database: connection,
        cipher: AesGcm::new(&GenericArray::clone_from_slice(&config.cookie_secret)),
    });

    let users = User::list(&global.database).await.context("list users")?;

    // Generate admin account if there are no users
    if users.len() == 0 {
        const ADMIN_USERNAME: &str = "admin";
        let password = generate_password(16);
        let password_hash = hash_password(&password)?;
        let admin_user = User::new(ADMIN_USERNAME, password_hash);
        admin_user.insert(&global.database).await.context("create admin user")?;

        tracing::info!(
            "Created user with username: {} and password: {}",
            ADMIN_USERNAME,
            password
        )
    }

    let server = create_dns_server(global.clone()).await?;

    let global_clone = global.clone();
    let server_clone = server.clone();
    tokio::spawn(async move { update_server_state_on_config_changes(global_clone, server_clone).await });

    tokio::select! {
        r = serve_web(config.http_server_address, global.clone()) => {
            if let Err(e) = r {
                tracing::error!("HTTP server exited with error: {}", e);
            }
        }
        r = metrics_service.run() => {
            if let Err(e) = r {
                tracing::error!("Metrics exited with error: {}", e);
            }
        },
        r = server.serve_tcp(config.dns_server_address) => {
            if let Err(e) = r {
                tracing::error!("TCP listener exited with error: {}", e);
            }
        },
        r = server.serve_udp(config.dns_server_address) => {
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
