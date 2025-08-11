use std::sync::Arc;

use crate::{blocklist::service::BlocklistService, cache::service::CacheService};

pub struct Services {
    pub cache: Arc<CacheService>,
    pub blocklist: Arc<BlocklistService>,
}

impl Services {
    pub fn new(cache_service: CacheService, blocklist_service: BlocklistService) -> Self {
        Self {
            cache: Arc::new(cache_service),
            blocklist: Arc::new(blocklist_service),
        }
    }
}
