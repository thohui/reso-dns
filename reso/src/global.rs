use std::sync::Arc;

use reso_cache::DnsMessageCache;

use crate::blocklist::service::BlocklistService;

/// Global state shared across all requests.
pub struct Global {
    pub cache: Arc<DnsMessageCache>,
    pub blocklist: Arc<BlocklistService>,
}

impl Global {
    pub fn new(cache: DnsMessageCache, blocklist_service: BlocklistService) -> Self {
        Self {
            cache: Arc::new(cache),
            blocklist: Arc::new(blocklist_service),
        }
    }
}
