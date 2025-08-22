use std::sync::Arc;

use reso_cache::MessageCache;

use crate::blocklist::service::BlocklistService;

pub struct Global {
    pub cache: Arc<MessageCache>,
    pub blocklist: Arc<BlocklistService>,
}

impl Global {
    pub fn new(cache: MessageCache, blocklist_service: BlocklistService) -> Self {
        Self {
            cache: Arc::new(cache),
            blocklist: Arc::new(blocklist_service),
        }
    }
}
