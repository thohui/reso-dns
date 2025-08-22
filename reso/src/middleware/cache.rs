use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::CacheKey;
use reso_context::{DnsMiddleware, DnsRequestCtx};

use crate::{global::Global, local::Local};

pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

        let cache_key = CacheKey::from_message(message)?;
        if let Some(cached_response) = ctx.global().cache.lookup(&cache_key).await {
            let mut local = ctx.local_mut();
            local.cache_hit = true;
            return Ok(Some(cached_response.into_custom_response(message.id)));
        }
        Ok(None)
    }
}
