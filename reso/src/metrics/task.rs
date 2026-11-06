use std::{net::SocketAddr, sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::time::{self, MissedTickBehavior};

use crate::{
    database::{
        CoreDatabasePool, MetricsDatabasePool,
        models::{activity_log, client, client_metrics, domain_metrics},
    },
    reverse::{self, HostnameCheck},
    services::config::ConfigReceiver,
};

pub const MINUTE_MS: i64 = 60_000;
pub const HOUR_MS: i64 = 3_600_000;
pub const DAY_MS: i64 = 86_400_000;

/// Buckets older than this are rolled up from 1-minute to 1-hour buckets.
pub const COMPRESS_TO_HOUR_AFTER_MS: i64 = 24 * HOUR_MS;
/// Buckets older than this are rolled up from 1 hour to 1 day buckets.
pub const COMPRESS_TO_DAY_AFTER_MS: i64 = 31 * DAY_MS;
pub const CLIENT_RETENTION_MS: i64 = 30 * DAY_MS;

const HOSTNAME_RECHECK_AFTER_MS: i64 = 6 * HOUR_MS;
const HOSTNAME_RETRY_NOT_FOUND_AFTER_MS: i64 = 30 * MINUTE_MS;
const HOSTNAME_RETRY_FAILED_AFTER_MS: i64 = 10 * MINUTE_MS;
const HOSTNAME_LOOKUP_TIMEOUT: Duration = Duration::from_secs(1);

/// Task that periodically truncates old activity logs to save space.
pub async fn run_metrics_truncation(
    db: Arc<MetricsDatabasePool>,
    mut config_rx: ConfigReceiver,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let (mut enabled, mut retention_secs, mut interval_secs) = {
        let cfg = config_rx.borrow_and_update();
        (
            cfg.logs.truncation_enabled,
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

                let cutoff = crate::time::now_millis() - Duration::from_secs(retention_secs).as_millis() as i64;

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
                    (cfg.logs.truncation_enabled, cfg.logs.retention_secs, cfg.logs.truncate_interval_secs.max(60))
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

pub async fn run_client_pruning(db: Arc<CoreDatabasePool>, shutdown: tokio_util::sync::CancellationToken) {
    let mut tick = time::interval(Duration::from_secs(3600));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let cutoff = crate::time::now_millis() - CLIENT_RETENTION_MS;
                match client::delete_before(&db, cutoff).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!("pruned {n} clients not seen in {} days", CLIENT_RETENTION_MS / DAY_MS),
                    Err(e) => tracing::error!("failed to prune clients: {}", e),
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down client pruning task");
                break;
            }
        }
    }
}

pub async fn run_client_hostname_resolution(
    db: Arc<CoreDatabasePool>,
    config_rx: ConfigReceiver,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let mut tick = time::interval(Duration::from_mins(2));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_servers: Option<Vec<SocketAddr>> = None;

    loop {
        tokio::select! {
            _ = async {
                tick.tick().await;
                let clients = config_rx.borrow().clients.clone();
                let servers = clients.resolved_ptr_servers();
                let disabled = clients.naming_disabled();
                if servers.is_empty() && !disabled {
                    tracing::debug!("no default gateway to ask for client hostnames yet, skipping");
                    return;
                }
                // naming may have been disabled before a restart, without this tick ever seeing the change
                let changed = match &last_servers {
                    Some(last) => *last != servers,
                    None => disabled,
                };

                if changed {
                    match client::reset_hostname_checks(&db, disabled).await {
                        Ok(_) => tracing::info!("ptr servers changed, looking up all client hostnames again"),
                        Err(e) => {
                            tracing::error!("failed to reset client hostname checks: {}", e);
                            return;
                        }
                    }
                }
                last_servers = Some(servers.clone());
                resolve_due_hostnames(&db, &servers).await;
            } => {}
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down client hostname resolution task");
                break;
            }
        }
    }
}

async fn resolve_due_hostnames(db: &CoreDatabasePool, servers: &[SocketAddr]) {
    if servers.is_empty() {
        return;
    }

    let now = crate::time::now_millis();
    let due = match client::due_for_hostname_check(db, now - HOSTNAME_RECHECK_AFTER_MS, 128).await {
        Ok(due) => due,
        Err(e) => {
            tracing::error!("failed to load clients for hostname lookup: {}", e);
            return;
        }
    };

    if due.is_empty() {
        tracing::debug!("no hostnames are due for lookup, skipping");
        return;
    }

    let checks: Vec<(String, HostnameCheck)> = futures::stream::iter(due)
        .map(|ip| async move {
            let check = match ip.parse() {
                Ok(addr) if reverse::is_private_ip(addr) => {
                    reverse::lookup_ptr(servers, addr, HOSTNAME_LOOKUP_TIMEOUT).await
                }
                _ => HostnameCheck::Skipped,
            };
            (ip, check)
        })
        .buffer_unordered(8) // run at most 8 lookups at the same time
        .collect()
        .await;

    let found = checks
        .iter()
        .filter(|(_, c)| matches!(c, HostnameCheck::Found(_)))
        .count();
    let total = checks.len();

    let not_found_checked_at = now - HOSTNAME_RECHECK_AFTER_MS + HOSTNAME_RETRY_NOT_FOUND_AFTER_MS;
    let failed_checked_at = now - HOSTNAME_RECHECK_AFTER_MS + HOSTNAME_RETRY_FAILED_AFTER_MS;
    match client::batch_set_hostnames(db, checks, now, not_found_checked_at, failed_checked_at).await {
        Ok(()) => tracing::debug!("resolved {found}/{total} client hostnames"),
        Err(e) => tracing::error!("failed to store client hostnames: {}", e),
    }
}
