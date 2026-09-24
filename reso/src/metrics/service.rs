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
use crate::{
    database::{
        MetricsDatabasePool,
        models::{
            activity_log::{self, ActivityLog},
            client_metrics::{self, ClientMetrics},
            domain_metrics::{self, DomainMetrics},
        },
    },
    services::config::ConfigReceiver,
    time::now_millis,
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
    config_rx: ConfigReceiver,
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
        let metrics = client_metrics::metrics_totals(db, None, 0).await?;
        Ok(Self {
            query: Arc::new(RwLock::new(LiveStats {
                total: metrics.total_count.max(0) as usize,
                blocked: metrics.blocked_count.max(0) as usize,
                cached: metrics.cached_count.max(0) as usize,
                errors: metrics.error_count.max(0) as usize,
                sum_duration: metrics.sum_duration.max(0) as u128,
                live_since: now_millis() as u128,
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
        config_rx: ConfigReceiver,
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
                config_rx,
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

        let mut client_map: HashMap<(i64, &str), ClientMetrics> = HashMap::with_capacity(self.batch.len());
        let mut domain_map: HashMap<(i64, &str), DomainMetrics> = HashMap::with_capacity(self.batch.len());

        for event in &self.batch {
            // floor to nearest bucket interval
            let bucket_ts = (event.ts_ms / Self::BUCKET_INTERVAL_MS) * Self::BUCKET_INTERVAL_MS;
            let blocked = event.blocked == Some(true);

            let m = client_map
                .entry((bucket_ts, event.client.as_str()))
                .or_insert_with(|| ClientMetrics::empty(bucket_ts, event.client.clone()));

            m.total_count += 1;
            m.blocked_count += blocked as i64;
            m.cached_count += (event.cache_hit == Some(true)) as i64;
            m.error_count += (event.kind == "error") as i64;
            m.sum_duration += event.dur_ms;

            if let Some(qname) = &event.qname {
                let m = domain_map
                    .entry((bucket_ts, qname.as_str()))
                    .or_insert_with(|| DomainMetrics::empty(bucket_ts, qname.clone()));
                m.total_count += 1;
                m.blocked_count += blocked as i64;
            }
        }

        // we purposely don't use tokio::join here as it doesn't matter for sqlite,
        // because sqlite only allows one write at a time.

        let client_buckets: Vec<_> = client_map.into_values().collect();
        let domain_buckets: Vec<_> = domain_map.into_values().collect();
        let (client_len, domain_len) = (client_buckets.len(), domain_buckets.len());

        match client_metrics::batch_upsert(&self.connection, client_buckets).await {
            Ok(()) => tracing::debug!("flushed {client_len} client metric buckets"),
            Err(e) => tracing::error!("failed to upsert client metrics: {}", e),
        }

        match domain_metrics::batch_upsert(&self.connection, domain_buckets).await {
            Ok(()) => tracing::debug!("flushed {domain_len} domain metric buckets"),
            Err(e) => tracing::error!("failed to upsert domain metrics: {}", e),
        }

        if self.config_rx.borrow().logs.enabled {
            match activity_log::batch_insert(&self.connection, &self.batch).await {
                Ok(()) => tracing::debug!("flushed {} activity logs", self.batch.len()),
                Err(e) => tracing::error!("failed to insert activity logs: {}", e),
            }
        }

        self.batch.clear();

        // during high loads, it's possible for the batch to grow outside of the original buffer capacity.
        // this is fine, but we want to shrink it back down to save memory once the load subsides.
        if self.batch.capacity() >= self.buffer_size.saturating_mul(2) {
            self.batch.shrink_to(self.buffer_size);
        }
    }
}
