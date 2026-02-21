use std::sync::Arc;

use aes_gcm::Aes256Gcm;
use reso_cache::DnsMessageCache;

use crate::{
    database::DatabaseConnection,
    metrics::service::{MetricsHandle, Stats},
    services::{blocklist::BlocklistService, config::ConfigService},
};

/// Global state shared across all requests.
pub type SharedGlobal = Arc<Global>;

pub struct Global {
    pub cache: DnsMessageCache,
    pub blocklist: BlocklistService,
    pub metrics: MetricsHandle,
    pub config_service: ConfigService,
    pub stats: Stats,
    pub database: Arc<DatabaseConnection>,
    pub cipher: Aes256Gcm,
}
