use async_trait::async_trait;
use bytes::Bytes;

use crate::{cache::service::CacheKey, resolver::DnsRequestCtx};

use super::DnsMiddleware;

pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;
        let cache_key = CacheKey::from_message(message)?;
        if let Some(cached_response) = ctx.cache_service().lookup(&cache_key).await {
            return Ok(Some(cached_response.into_custom_response(message.id)));
        }
        Ok(None)
    }
}
