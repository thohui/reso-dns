use std::{sync::Arc, time::Duration};

use tokio::time::{self, MissedTickBehavior};

use crate::{
    database::{
        MetricsDatabasePool,
        models::{activity_log::ActivityLog, client_metrics::ClientMetrics, domain_metrics::DomainMetrics},
    },
    services::config::model::Config,
};

/// Task that periodically truncates old activity logs to save space.
pub async fn run_metrics_truncation(
    db: Arc<MetricsDatabasePool>,
    mut config_rx: tokio::sync::watch::Receiver<Arc<Config>>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let (mut enabled, mut retention_secs, mut interval_secs) = {
        let cfg = config_rx.borrow_and_update();
        (
            cfg.logs.enabled,
            cfg.logs.retention_secs,
            cfg.logs.truncate_interval_secs.max(60),
        )
    };

    tracing::info!(
        "running metrics truncation (enabled={}, retention={}s, interval={}s)",
        enabled,
        retention_secs,
        interval_secs,
    );

    let mut tick = time::interval(Duration::from_secs(interval_secs));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tick.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if !enabled {
                    continue;
                }

                let retention = Duration::from_secs(retention_secs);
                let cutoff = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    - retention.as_millis() as i64;

                if let Err(e) = ActivityLog::delete_before(&db, cutoff).await {
                    tracing::error!("failed to truncate old activity logs: {}", e);
                    continue;
                }

                if let Err(e) = db
                    .interact(|c| c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);"))
                    .await
                {
                    tracing::error!("failed to checkpoint metrics WAL after truncation: {}", e);
                } else {
                    tracing::info!("truncated activity logs older than {}s", retention_secs);
                }
            }
            Ok(()) = config_rx.changed() => {
                let (new_enabled, new_retention, new_interval) = {
                    let cfg = config_rx.borrow_and_update();
                    (cfg.logs.enabled, cfg.logs.retention_secs, cfg.logs.truncate_interval_secs.max(60))
                };

                if new_enabled != enabled {
                    enabled = new_enabled;
                    tracing::info!("logs truncation {}", if enabled { "enabled" } else { "disabled" });
                }

                if new_interval != interval_secs {
                    interval_secs = new_interval;
                    tick = time::interval(Duration::from_secs(interval_secs));
                    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    tick.tick().await;
                    tracing::info!("logs truncation interval updated to {}s", interval_secs);
                }

                if new_retention != retention_secs {
                    retention_secs = new_retention.max(1); // enforce minimum 1s retention
                    tracing::info!("logs retention updated to {}s", retention_secs);
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down logs truncation task");
                break;
            }
        }
    }
}

/// Rolls up aggregate metrics older than this threshold from 1 minute to 1 hour buckets.
const COMPRESS_AFTER_SECS: u64 = 24 * 3600;

/// Task that periodically compresses old metrics into larger time buckets to save space.
pub async fn run_metrics_compression(db: Arc<MetricsDatabasePool>, shutdown: tokio_util::sync::CancellationToken) {
    let mut tick = time::interval(Duration::from_secs(3600));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // we purposely don't await the first tick so that we compress on startup,
    // in case there are already old metrics that need compressing before the first hourly tick.

    tracing::info!("running metrics compression (compress_after={}s)", COMPRESS_AFTER_SECS);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let cutoff = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    - Duration::from_secs(COMPRESS_AFTER_SECS).as_millis() as i64;

                // we purposefully don't run these compressions in parallel since sqlite can only have one writer at a time.

                if let Err(e) = ClientMetrics::compress_before(&db, cutoff).await {
                    tracing::error!("failed to compress client metrics: {}", e);
                }
                if let Err(e) = DomainMetrics::compress_before(&db, cutoff).await {
                    tracing::error!("failed to compress domain metrics: {}", e);
                }

                if let Err(e) = db
                    .interact(|c| c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);"))
                    .await
                {
                    tracing::error!("failed to checkpoint metrics WAL after truncation: {}", e);
                } else {
                    tracing::info!("compressed aggregated metrics older than {}s", COMPRESS_AFTER_SECS);
                }

            }
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down metrics compression task");
                break;
            }
        }
    }
}
