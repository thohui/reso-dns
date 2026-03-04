use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::{CacheKey, CacheResult, NegKind};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessage, DnsMessageBuilder, DnsOpcode, DnsResponseCode, Edns, message::EdnsOptionCode};

use crate::{global::Global, local::Local, middleware::echo_edns};

fn cache_response_flags(query: &DnsMessage) -> DnsFlags {
    DnsFlags::new(
        true,
        DnsOpcode::Query,
        false,
        false,
        query.flags.recursion_desired,
        true,
        false,
        query.flags.checking_disabled,
    )
}

/// Caching middleware that serves responses from cache if available.
pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

        // Skip cache if the query has EDNS Client Subnet, since that would
        // require geo-aware caching which is out of scope.
        let has_ecs = message
            .edns()
            .as_ref()
            .map(|e| e.options.iter().any(|o| o.code == EdnsOptionCode::ClientSubnet))
            .unwrap_or(false);

        if has_ecs {
            return Ok(None);
        }

        let cache_key = CacheKey::try_from(message)?;

        match ctx.global().cache.lookup(&cache_key).await {
            CacheResult::Negative(result) => {
                tracing::debug!("negative cache hit for {:?} {:?}", cache_key, result);
                ctx.local_mut().cache_hit = true;

                let response_code = match result.kind {
                    NegKind::NxDomain => DnsResponseCode::NxDomain,
                    NegKind::NoData => DnsResponseCode::NoError,
                };

                let builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(cache_response_flags(message))
                    .with_response(response_code)
                    .with_questions(message.questions().to_vec())
                    .with_authority_records(vec![result.soa_record]);

                Ok(Some(echo_edns(message, builder).build().encode()?))
            }

            CacheResult::Positive { records, ttl } => {
                tracing::debug!("cache hit for {:?}", cache_key);
                ctx.local_mut().cache_hit = true;

                let answers: Vec<_> = records
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        r.ttl = ttl;
                        r
                    })
                    .collect();

                let builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(cache_response_flags(message))
                    .with_questions(message.questions().to_vec())
                    .with_answers(answers);

                Ok(Some(echo_edns(message, builder).build().encode()?))
            }

            CacheResult::Miss => Ok(None),
        }
    }
}
