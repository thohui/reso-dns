use std::{sync::Arc, time::Duration};

use tokio::time::{self, MissedTickBehavior};

use crate::{
    database::MetricsDatabasePool, database::models::activity_log::ActivityLog, services::config::model::Config,
};

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
