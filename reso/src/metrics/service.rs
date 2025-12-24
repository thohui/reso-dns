use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::{
    sync::{
        RwLock,
        mpsc::{self, Receiver, Sender},
    },
    time::{self, MissedTickBehavior},
};

use super::event::QueryLogEvent;
use crate::database::{DatabaseConnection, models::query_log::DnsQueryLog};

pub enum MetricsMessage {
    Shutdown,
    Event(QueryLogEvent),
}

/// Service for handling metrics.
pub struct MetricsService {
    connection: Arc<DatabaseConnection>,
    rx: Receiver<MetricsMessage>,
    batch: Vec<QueryLogEvent>,

    live_stats: Arc<RwLock<LiveStats>>,
}

#[derive(Clone)]
pub struct MetricsHandle(Sender<MetricsMessage>);

impl MetricsHandle {
    pub fn shutdown(&self) -> anyhow::Result<()> {
        let _ = self.0.try_send(MetricsMessage::Shutdown);
        Ok(())
    }

    pub fn record(&self, event: QueryLogEvent) -> anyhow::Result<()> {
        let _ = self.0.try_send(MetricsMessage::Event(event));
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveStats {
    total: usize,
    blocked: usize,
    cached: usize,
}

impl LiveStats {
    fn apply(&mut self, stats: &QueryLogEvent) {
        self.total += 1;
        self.blocked += if stats.blocked { 1 } else { 0 };
        self.cached += if stats.cache_hit { 1 } else { 0 }
    }
}

pub struct Stats {
    live: Arc<RwLock<LiveStats>>,
}

impl Stats {
    pub async fn live(&self) -> LiveStats {
        let stats = self.live.read().await;
        stats.clone()
    }
}

impl MetricsService {
    pub fn new(connection: Arc<DatabaseConnection>, buffer: usize) -> (MetricsHandle, Stats, Self) {
        let live = Arc::new(RwLock::new(LiveStats {
            blocked: 0,
            cached: 0,
            total: 0,
        }));

        let (tx, rx) = mpsc::channel::<MetricsMessage>(buffer);
        (
            MetricsHandle(tx),
            Stats { live: live.clone() },
            Self {
                connection,
                rx,
                batch: Vec::with_capacity(buffer),
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
                _ = tick.tick() => {
                    self.flush().await?;
                }

                msg = self.rx.recv() => {
                    match msg {
                        None | Some(MetricsMessage::Shutdown) => {
                            tracing::info!("shutting down metrics service");
                            self.flush().await?;
                            break;
                        }
                        Some(MetricsMessage::Event(ev)) => self.on_event(ev).await
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_event(&mut self, event: QueryLogEvent) {
        {
            let mut write = self.live_stats.write().await;
            write.apply(&event);
        }
        self.batch.push(event);
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        if self.batch.is_empty() {
            return Ok(());
        }

        // swap out so we can keep receiving while writing
        let rows = std::mem::take(&mut self.batch);

        let db_rows: Vec<DnsQueryLog> = rows.into_iter().map(|r| r.into_db_model()).collect();

        DnsQueryLog::batch_insert(&self.connection, &db_rows).await?;

        tracing::info!("flushed {} events to the database", db_rows.len());

        Ok(())
    }
}
