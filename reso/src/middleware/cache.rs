use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::{CacheKey, CacheResult, NegKind};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessageBuilder, DnsOpcode, DnsResponseCode};

use crate::{global::Global, local::Local};

/// Caching middleware that serves responses from cache if available.
pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

        // skip the cache if the query uses edns for now.
        if message.edns().as_ref().is_some() {
            return Ok(None);
        }

        let cache_key = CacheKey::try_from(message)?;
        match ctx.global().cache.lookup(&cache_key).await {
            CacheResult::Negative(result) => {
                tracing::debug!("negative cache hit for {:?} {:?}", cache_key, result);

                let mut local = ctx.local_mut();
                local.cache_hit = true;

                let response_code = match result.kind {
                    NegKind::NxDomain => DnsResponseCode::NxDomain,
                    NegKind::NoData => DnsResponseCode::NoError,
                };

                let flags = DnsFlags::new(
                    true,
                    DnsOpcode::Query,
                    false,
                    false,
                    message.flags.recursion_desired,
                    true,
                    false,
                    message.flags.checking_disabled,
                );

                let message = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(flags)
                    .with_response(response_code)
                    .with_questions(message.questions().to_vec())
                    .with_authority_records(vec![result.soa_record])
                    .build();

                let bytes = message.encode()?;

                Ok(Some(bytes))
            }

            CacheResult::Positive(recs) => {
                tracing::debug!("cache hit for {:?}", cache_key);
                let mut local = ctx.local_mut();
                local.cache_hit = true;
                let flags = DnsFlags::new(
                    true,
                    DnsOpcode::Query,
                    false,
                    false,
                    message.flags.recursion_desired,
                    true,
                    false,
                    message.flags.checking_disabled,
                );
                let message = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(flags)
                    .with_questions(message.questions().to_vec())
                    .with_answers(recs.to_vec())
                    .build();

                let bytes = message.encode()?;
                Ok(Some(bytes))
            }
            CacheResult::Miss => Ok(None),
        }
    }
}
