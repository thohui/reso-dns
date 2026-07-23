use std::{sync::Arc, time::Duration};

use tokio::time::{self, MissedTickBehavior};

use crate::{
    database::{
        MetricsDatabasePool,
        models::{activity_log, client_metrics, domain_metrics},
    },
    services::config::Config,
};

pub const MINUTE_MS: i64 = 60_000;
pub const HOUR_MS: i64 = 3_600_000;
pub const DAY_MS: i64 = 86_400_000;

/// Buckets older than this are rolled up from 1-minute to 1-hour buckets.
pub const COMPRESS_TO_HOUR_AFTER_MS: i64 = 24 * HOUR_MS;
/// Buckets older than this are rolled up from 1 hour to 1 day buckets.
pub const COMPRESS_TO_DAY_AFTER_MS: i64 = 31 * DAY_MS;

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

                if let Err(e) = activity_log::delete_before(&db, cutoff).await {
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

/// Task that periodically compresses old metrics into larger time buckets to save space.
/// Rolls 1-minute buckets into 1-hour buckets after `COMPRESS_TO_HOUR_AFTER_MS`, then
/// 1-hour buckets into 1-day buckets after `COMPRESS_TO_DAY_AFTER_MS`.
pub async fn run_metrics_compression(db: Arc<MetricsDatabasePool>, shutdown: tokio_util::sync::CancellationToken) {
    let mut tick = time::interval(Duration::from_secs(3600));

    // Don't await the first tick in case there are already metrics that need comrpessing before
    // the first hourly tick.
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        "running metrics compression (compress_to_hour_after={}s, compress_to_day_after={}s)",
        COMPRESS_TO_HOUR_AFTER_MS / 1000,
        COMPRESS_TO_DAY_AFTER_MS / 1000,
    );

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let now = crate::time::now_millis();
                let hour_cutoff = now - COMPRESS_TO_HOUR_AFTER_MS;
                let day_cutoff = now - COMPRESS_TO_DAY_AFTER_MS;

                // we purposely don't run these compressions in parallel since sqlite can only have one writer at a time.

                if let Err(e) = client_metrics::compress_before(&db, hour_cutoff, HOUR_MS).await {
                    tracing::error!("failed to compress client metrics to hourly buckets: {}", e);
                }
                if let Err(e) = domain_metrics::compress_before(&db, hour_cutoff, HOUR_MS).await {
                    tracing::error!("failed to compress domain metrics to hourly buckets: {}", e);
                }
                if let Err(e) = client_metrics::compress_before(&db, day_cutoff, DAY_MS).await {
                    tracing::error!("failed to compress client metrics to daily buckets: {}", e);
                }
                if let Err(e) = domain_metrics::compress_before(&db, day_cutoff, DAY_MS).await {
                    tracing::error!("failed to compress domain metrics to daily buckets: {}", e);
                }

                if let Err(e) = db
                    .interact(|c| c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);"))
                    .await
                {
                    tracing::error!("failed to checkpoint metrics WAL after truncation: {}", e);
                } else {
                    tracing::info!("compressed aggregated metrics");
                }

            }
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down metrics compression task");
                break;
            }
        }
    }
}
