use std::sync::Arc;

use arc_swap::ArcSwap;
use model::Config;

use database::models::config::ConfigSetting;

use crate::database::{self, DatabaseConnection};

pub mod model;

/// Service for managing the server configuration
pub struct ConfigService {
    db: Arc<DatabaseConnection>,
    config: ArcSwap<Config>,
    tx: tokio::sync::watch::Sender<Arc<Config>>,
    _rx_guard: tokio::sync::watch::Receiver<Arc<Config>>,
}

impl ConfigService {
    /// Initializes the `ConfigService`
    pub async fn initialize(db: Arc<DatabaseConnection>) -> anyhow::Result<ConfigService> {
        let config = Self::initialize_config(&db).await?;
        let config = Arc::new(config);
        let (tx, rx) = tokio::sync::watch::channel(config.clone());
        Ok(ConfigService {
            db,
            config: ArcSwap::new(config),
            tx,
            _rx_guard: rx,
        })
    }

    /// Initializes the configuration from the database.
    async fn initialize_config(db: &DatabaseConnection) -> anyhow::Result<Config> {
        let map = ConfigSetting::all(db).await?;

        if map.is_empty() {
            tracing::info!("Initializing database config");
            let default_config = Config::default();
            ConfigSetting::batch_set(db, default_config.to_kv()).await?;
            Ok(default_config)
        } else {
            Ok(Config::from_kv(&map))
        }
    }

    /// Updates the configuration and notify the subscribers.
    pub async fn update_config(&self, config: Config) -> anyhow::Result<()> {
        ConfigSetting::batch_set(&self.db, config.to_kv()).await?;
        let arc_config = Arc::new(config);
        self.config.store(arc_config.clone());
        self.tx.send_replace(arc_config);
        Ok(())
    }

    /// Gets the config.
    pub fn get_config(&self) -> Arc<Config> {
        self.config.load_full()
    }

    /// Subscribes to any config changes.
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<Config>> {
        self.tx.subscribe()
    }
}
