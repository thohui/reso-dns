use std::sync::Arc;

use aes_gcm::Aes256Gcm;
use reso_cache::DnsMessageCache;

use crate::{
    database::{CoreDatabasePool, MetricsDatabasePool},
    metrics::service::{MetricsHandle, Stats},
    services::{config::ConfigService, domain_rules::DomainRulesService, local_records::LocalRecordService},
};

/// Global state shared across all requests.
pub type SharedGlobal = Arc<Global>;

pub struct Global {
    pub cache: DnsMessageCache,
    pub domain_rules: DomainRulesService,
    pub local_records: LocalRecordService,
    pub metrics: MetricsHandle,
    pub config: ConfigService,
    pub stats: Stats,
    pub core_database: Arc<CoreDatabasePool>,
    pub metrics_database: Arc<MetricsDatabasePool>,
    pub cipher: Aes256Gcm,
}
