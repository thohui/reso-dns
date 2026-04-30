use std::{collections::HashMap, sync::Arc, time::Duration};

use serde::Serialize;
use tokio::{
    sync::{
        RwLock,
        mpsc::{self, Receiver, Sender},
    },
    time::{self, MissedTickBehavior},
};

use super::event::{ErrorLogEvent, QueryLogEvent};
use crate::database::{
    MetricsDatabasePool,
    models::{activity_log::ActivityLog, client_metrics::ClientMetrics, domain_metrics::DomainMetrics},
};

pub enum MetricsMessage {
    #[allow(dead_code)]
    Shutdown,
    Query(QueryLogEvent),
    Error(ErrorLogEvent),
}

/// Service for handling metrics.
pub struct MetricsService {
    connection: Arc<MetricsDatabasePool>,
    rx: Receiver<MetricsMessage>,
    batch: Vec<ActivityLog>,
    buffer_size: usize,
    live_stats: Arc<RwLock<LiveStats>>,
}

#[derive(Clone)]
pub struct MetricsHandle(Sender<MetricsMessage>);

impl MetricsHandle {
    #[allow(dead_code)]
    pub fn shutdown(&self) {
        if let Err(e) = self.0.try_send(MetricsMessage::Shutdown) {
            tracing::error!("failed to send shutdown signal to metrics service {}", e)
        }
    }

    pub fn query(&self, event: QueryLogEvent) {
        if let Err(e) = self.0.try_send(MetricsMessage::Query(event)) {
            tracing::error!("failed to record query metric: {}", e)
        }
    }

    pub fn error(&self, error: ErrorLogEvent) {
        if let Err(e) = self.0.try_send(MetricsMessage::Error(error)) {
            tracing::error!("failed to record error metric: {}", e)
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveStats {
    /// Total requests
    pub total: usize,
    /// Total queries blocked
    pub blocked: usize,
    /// Total queries cached
    pub cached: usize,
    /// Total errors
    pub errors: usize,
    /// Sum of the duration of all requests
    pub sum_duration: u128,
    /// Live since
    pub live_since: u128,
}

impl LiveStats {
    fn apply_event(&mut self, stats: &QueryLogEvent) {
        self.total += 1;
        self.blocked += if stats.blocked { 1 } else { 0 };
        self.cached += if stats.cache_hit { 1 } else { 0 };
        self.sum_duration += stats.dur_ms as u128
    }
    fn apply_error(&mut self, error: &ErrorLogEvent) {
        self.total += 1;
        self.errors += 1;
        self.sum_duration += error.dur_ms as u128;
    }
}

pub struct Stats {
    query: Arc<RwLock<LiveStats>>,
}

impl Stats {
    pub async fn init(db: &MetricsDatabasePool) -> anyhow::Result<Self> {
        let activity_stats = ActivityLog::fetch_stats(db).await?;
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Ok(Self {
            query: Arc::new(RwLock::new(LiveStats {
                total: activity_stats.total as usize,
                blocked: activity_stats.blocked as usize,
                cached: activity_stats.cached as usize,
                errors: activity_stats.errors as usize,
                sum_duration: activity_stats.sum_duration as u128,
                live_since: ts_ms,
            })),
        })
    }
    pub async fn live(&self) -> LiveStats {
        let stats = self.query.read().await;
        stats.clone()
    }
}

impl MetricsService {
    pub async fn new(
        connection: Arc<MetricsDatabasePool>,
        buffer_size: usize,
    ) -> anyhow::Result<(MetricsHandle, Stats, Self)> {
        let live = Stats::init(&connection).await?;

        let (tx, rx) = mpsc::channel::<MetricsMessage>(buffer_size);
        Ok((
            MetricsHandle(tx),
            Stats {
                query: live.query.clone(),
            },
            Self {
                connection,
                rx,
                batch: Vec::with_capacity(buffer_size),
                buffer_size,
                live_stats: live.query.clone(),
            },
        ))
    }

    /// Interval for bucketing metrics in milliseconds.
    const BUCKET_INTERVAL_MS: i64 = 60_000; // 1 min.

    pub async fn run(mut self, shutdown: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        tracing::info!("running metrics service");

        let mut tick = time::interval(Duration::from_secs(5));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        tick.tick().await;

        loop {
            tokio::select! {
                _ = tick.tick() =>  {
                    self.flush_events().await;
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("shutting down metrics service");

                    // drain any buffered messages before flushing
                    while let Ok(msg) = self.rx.try_recv() {
                        match msg {
                            MetricsMessage::Query(ev) => {
                                self.live_stats.write().await.apply_event(&ev);
                                self.batch.push(ev.into_db_model());
                            },
                            MetricsMessage::Error(ev) => {
                                self.live_stats.write().await.apply_error(&ev);
                                self.batch.push(ev.into_db_model());
                            },
                            MetricsMessage::Shutdown => break,
                        }
                    }

                    self.flush_events().await;
                    break;
                },
                msg = self.rx.recv() => {
                    match msg {
                        None | Some(MetricsMessage::Shutdown) => {
                            tracing::info!("shutting down metrics service");
                            self.flush_events().await;
                            break;
                        },
                        Some(MetricsMessage::Query(ev)) => {
                            self.live_stats.write().await.apply_event(&ev);
                            self.batch.push(ev.into_db_model());
                        },
                        Some(MetricsMessage::Error(ev)) => {
                            self.live_stats.write().await.apply_error(&ev);
                            self.batch.push(ev.into_db_model());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn flush_events(&mut self) {
        if self.batch.is_empty() {
            return;
        }

        let mut client_map: HashMap<(i64, String), ClientMetrics> = HashMap::with_capacity(self.batch.len());
        let mut domain_map: HashMap<(i64, String), DomainMetrics> = HashMap::with_capacity(self.batch.len());

        for event in &self.batch {
            // floor to nearest bucket interval
            let bucket_ts = (event.ts_ms / Self::BUCKET_INTERVAL_MS) * Self::BUCKET_INTERVAL_MS;

            let is_error = event.kind == "error";
            let client_metrics = ClientMetrics {
                bucket_ts,
                client: event.client.clone(),
                total_count: 1,
                blocked_count: if event.blocked == Some(true) { 1 } else { 0 },
                cached_count: if event.cache_hit == Some(true) { 1 } else { 0 },
                error_count: if is_error { 1 } else { 0 },
                sum_duration: event.dur_ms as i64,
            };

            client_map
                .entry((bucket_ts, event.client.clone()))
                .and_modify(|m| m.merge(&client_metrics))
                .or_insert(client_metrics);

            if let Some(qname) = &event.qname {
                let domain_metrics = DomainMetrics {
                    blocked_count: if event.blocked == Some(true) { 1 } else { 0 },
                    bucket_ts,
                    qname: qname.clone(),
                    total_count: 1,
                };

                domain_map
                    .entry((bucket_ts, qname.clone()))
                    .and_modify(|m| m.merge(&domain_metrics))
                    .or_insert(domain_metrics);
            }
        }

        // we purposefully don't use tokio::join here as it doesn't matter for sqlite,
        // because sqlite only allows one write at a time.

        let client_buckets: Vec<_> = client_map.into_values().collect();
        let domain_buckets: Vec<_> = domain_map.into_values().collect();

        match ClientMetrics::batch_upsert(&self.connection, &client_buckets).await {
            Ok(()) => tracing::debug!("flushed {} client metric buckets", client_buckets.len()),
            Err(e) => tracing::error!("failed to upsert client metrics: {}", e),
        }

        match DomainMetrics::batch_upsert(&self.connection, &domain_buckets).await {
            Ok(()) => tracing::debug!("flushed {} domain metric buckets", domain_buckets.len()),
            Err(e) => tracing::error!("failed to upsert domain metrics: {}", e),
        }

        match ActivityLog::batch_insert(&self.connection, &self.batch).await {
            Ok(()) => tracing::debug!("flushed {} activity logs", self.batch.len()),
            Err(e) => tracing::error!("failed to insert activity logs: {}", e),
        }

        self.batch.clear();

        // during high loads, it's possible for the batch to grow outside of the original buffer capacity.
        // this is fine, but we want to shrink it back down to save memory once the load subsides.
        if self.batch.capacity() >= self.buffer_size.saturating_mul(2) {
            self.batch.shrink_to(self.buffer_size);
        }
    }
}
