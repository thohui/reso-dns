use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::{CacheKey, CacheResult};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessageBuilder};

use crate::{global::Global, local::Local};

pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;
        let cache_key = CacheKey::from_message(message)?;
        match ctx.global().cache.lookup(&cache_key).await {
            CacheResult::Miss => Ok(None),
            CacheResult::Negative(_) => Ok(None), // todo: implement
            CacheResult::Positive(recs) => {
                tracing::debug!("cache hit for {:?}", cache_key);
                let mut local = ctx.local_mut();
                local.cache_hit = true;
                let message = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(DnsFlags {
                        qr: true,
                        rd: message.flags.rd,
                        cd: message.flags.cd,
                        ..Default::default()
                    })
                    .with_questions(message.questions().to_vec())
                    .with_answers(recs.to_vec())
                    .build();

                let bytes = message.encode()?;
                Ok(Some(bytes))
            }
        }
    }
}
