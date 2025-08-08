use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;

use crate::{cache::service::CacheService, resolver::DnsRequestCtx};

use super::DnsMiddleware;

pub struct CacheMiddleware {
    service: Arc<CacheService>,
}

#[async_trait]
impl DnsMiddleware for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>> {
        Ok(None)
    }
}
