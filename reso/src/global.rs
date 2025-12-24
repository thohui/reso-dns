use reso_cache::DnsMessageCache;

use crate::{
    blocklist::service::BlocklistService,
    metrics::service::{MetricsHandle, Stats},
};

/// Global state shared across all requests.
pub struct Global {
    pub cache: DnsMessageCache,
    pub blocklist: BlocklistService,
    pub metrics: MetricsHandle,
    pub stats: Stats,
}
