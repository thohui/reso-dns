use async_trait::async_trait;
use reso_cache::{CacheKey, CacheResult, NegKind};
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsFlags, DnsMessage, DnsMessageBuilder, DnsOpcode, DnsResponseCode, message::EdnsOptionCode};

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
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
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

        let mut cache_hit = false;

        let resp = match ctx.global().cache.lookup(&cache_key).await {
            CacheResult::Negative(result) => {
                cache_hit = true;
                let response_code = match result.kind {
                    NegKind::NxDomain => DnsResponseCode::NxDomain,
                    NegKind::NoData => DnsResponseCode::NoError,
                };

                let builder = echo_edns(
                    message,
                    DnsMessageBuilder::new()
                        .with_id(message.id)
                        .with_flags(cache_response_flags(message))
                        .with_response(response_code)
                        .with_questions(message.questions().to_vec())
                        .with_authority_records(vec![result.soa_record]),
                );

                let bytes = builder.build().encode()?;
                Ok(Some(DnsResponse::from_bytes(bytes)))
            }

            CacheResult::Positive { records, ttl } => {
                cache_hit = true;

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

                let bytes = echo_edns(message, builder).build().encode()?;
                Ok(Some(DnsResponse::from_bytes(bytes)))
            }

            CacheResult::Miss => Ok(None),
        };

        ctx.local_mut().cache_hit = cache_hit;
        return resp;
    }

    async fn on_response(
        &self,
        ctx: &mut DnsRequestCtx<Global, Local>,
        response: &mut DnsResponse,
    ) -> anyhow::Result<()> {
        let message = ctx.message()?;

        let has_ecs = message
            .edns()
            .as_ref()
            .map(|e| e.options.iter().any(|o| o.code == EdnsOptionCode::ClientSubnet))
            .unwrap_or(false);

        let should_cache = !ctx.local().cache_hit && !has_ecs && !ctx.local().blocked && !ctx.local().rate_limited;

        if should_cache {
            let question = message.questions().first();
            let ttl = response.message()?.answers().iter().map(|a| a.ttl()).min().unwrap_or(0);

            tracing::debug!(
                "caching entry to question {:?} for {} seconds",
                question.map(|r| r.qname.as_str()),
                ttl
            );

            ctx.global().cache.insert(message, response.message()?).await;
        }

        Ok(())
    }
}
