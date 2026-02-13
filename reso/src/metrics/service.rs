use std::{sync::Arc, time::Duration};

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
    DatabaseConnection,
    models::{error_log::DnsErrorLog, query_log::DnsQueryLog},
};

pub enum MetricsMessage {
    #[allow(clippy::code)]
    Shutdown,
    Query(QueryLogEvent),
    Error(ErrorLogEvent),
}

/// Service for handling metrics.
pub struct MetricsService {
    connection: Arc<DatabaseConnection>,
    rx: Receiver<MetricsMessage>,
    query_batch: Vec<QueryLogEvent>,
    error_batch: Vec<ErrorLogEvent>,
    live_stats: Arc<RwLock<LiveStats>>,
}

#[derive(Clone)]
pub struct MetricsHandle(Sender<MetricsMessage>);

impl MetricsHandle {
    /// Signals the metrics service to shut down by sending a shutdown message.
    ///
    /// If sending the message fails (for example because the receiver was dropped), an error is logged.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::metrics::MetricsHandle;
    /// let handle: MetricsHandle = /* obtain handle */ unimplemented!();
    /// handle.shutdown();
    /// ```
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
    pub async fn live(&self) -> LiveStats {
        let stats = self.query.read().await;
        stats.clone()
    }
}

impl MetricsService {
    pub fn new(connection: Arc<DatabaseConnection>, buffer: usize) -> (MetricsHandle, Stats, Self) {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let live = Arc::new(RwLock::new(LiveStats {
            blocked: 0,
            cached: 0,
            total: 0,
            errors: 0,
            sum_duration: 0,
            live_since: ts_ms,
        }));

        let (tx, rx) = mpsc::channel::<MetricsMessage>(buffer);
        (
            MetricsHandle(tx),
            Stats { query: live.clone() },
            Self {
                connection,
                rx,
                query_batch: Vec::with_capacity(buffer),
                error_batch: Vec::with_capacity(buffer),
                live_stats: live,
            },
        )
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        tracing::info!("Running metrics service");

        let mut tick = time::interval(Duration::from_secs(5));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        tick.tick().await;

        loop {
            tokio::select! {
                _ = tick.tick() =>  self.flush_events().await,
                msg = self.rx.recv() => {
                    match msg {
                        None | Some(MetricsMessage::Shutdown) => {
                            tracing::info!("shutting down metrics service");
                            self.flush_events().await;
                            break;
                        },
                        Some(MetricsMessage::Query(ev)) => self.on_event(ev).await,
                        Some(MetricsMessage::Error(ev)) => self.on_error(ev).await
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_event(&mut self, event: QueryLogEvent) {
        {
            let mut write = self.live_stats.write().await;
            write.apply_event(&event);
        }
        self.query_batch.push(event);
    }

    async fn on_error(&mut self, error: ErrorLogEvent) {
        {
            let mut write = self.live_stats.write().await;
            write.apply_error(&error);
        }
        self.error_batch.push(error);
    }

    async fn flush_events(&mut self) {
        if let Err(e) = tokio::try_join!(self.flush_query_events(), self.flush_error_events()) {
            tracing::error!("failed to flush events to db: {}", e);
        }
        self.error_batch.clear();
        self.query_batch.clear();
    }

    async fn flush_query_events(&self) -> anyhow::Result<()> {
        if self.query_batch.is_empty() {
            return Ok(());
        }

        let db_rows: Vec<DnsQueryLog> = self.query_batch.iter().cloned().map(|r| r.into_db_model()).collect();

        DnsQueryLog::batch_insert(&self.connection, &db_rows).await?;

        tracing::debug!("flushed {} query events to the database", db_rows.len());

        Ok(())
    }

    async fn flush_error_events(&self) -> anyhow::Result<()> {
        if self.error_batch.is_empty() {
            return Ok(());
        }

        let db_rows: Vec<DnsErrorLog> = self.error_batch.iter().cloned().map(|r| r.into_db_model()).collect();

        DnsErrorLog::batch_insert(&self.connection, &db_rows).await?;

        tracing::debug!("flushed {} error events to the database", db_rows.len());

        Ok(())
    }
}

impl Drop for MetricsService {
    fn drop(&mut self) {
        futures::executor::block_on(self.flush_events())
    }
}