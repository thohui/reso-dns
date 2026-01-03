use std::sync::Arc;

use aes_gcm::Aes256Gcm;
use reso_cache::DnsMessageCache;

use crate::{
    blocklist::service::BlocklistService,
    config::Config,
    database::DatabaseConnection,
    metrics::service::{MetricsHandle, Stats},
};

/// Global state shared across all requests.
pub type SharedGlobal = Arc<Global>;

pub struct Global {
    pub cache: DnsMessageCache,
    pub blocklist: BlocklistService,
    pub metrics: MetricsHandle,
    pub stats: Stats,
    pub config: Config,
    pub database: Arc<DatabaseConnection>,
    pub cipher: Aes256Gcm,
}
