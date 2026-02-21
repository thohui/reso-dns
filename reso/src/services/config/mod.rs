use std::sync::Arc;

use arc_swap::ArcSwap;
use model::Config;

use database::models::config::Config as DbConfig;

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
            db: db,
            config: ArcSwap::new(config),
            tx: tx,
            _rx_guard: rx,
        })
    }

    /// Initializes the configuration from the database.
    async fn initialize_config(db: &DatabaseConnection) -> anyhow::Result<Config> {
        let db_config = DbConfig::get(&db).await?;

        // has the config data been initialized before?
        let config = if db_config.version() == 0 {
            tracing::info!("Initializing database config");
            let default_config = Config::default();
            let value = serde_json::to_string(&default_config)?;
            DbConfig::update_data(&db, value).await?;
            default_config
        } else {
            serde_json::from_str(&db_config.data)?
        };

        Ok(config)
    }

    /// Updates the configuration and notify the subscribers.
    pub async fn update_config(&self, config: Config) -> anyhow::Result<()> {
        let stringified_data = serde_json::to_string(&config)?;
        DbConfig::update_data(&self.db, stringified_data).await?;
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
