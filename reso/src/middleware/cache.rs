use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::CacheKey;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessageBuilder};

use crate::{global::Global, local::Local};

pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

        let cache_key = CacheKey::from_message(message)?;
        if let Some(recs) = ctx.global().cache.lookup(&cache_key).await {
            let mut local = ctx.local_mut();
            local.cache_hit = true;
            tracing::debug!("cache hit for {:?}", cache_key);
            let message = DnsMessageBuilder::new()
                .with_id(message.id)
                .with_flags(DnsFlags {
                    qr: true,
                    rd: message.flags.rd,
                    cd: message.flags.cd,
                    ..Default::default()
                })
                .with_questions(message.questions().to_vec())
                .with_answers(recs.iter().cloned().collect())
                .build();

            let bytes = message.encode()?;
            return Ok(Some(bytes));
        }
        Ok(None)
    }
}
