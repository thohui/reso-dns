use async_trait::async_trait;
use bytes::Bytes;
use reso_cache::{CacheKey, CacheResult, NegKind};
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessageBuilder, DnsOpcode, DnsResponseCode, Edns, message::EdnsOptionCode};

use crate::{global::Global, local::Local};

/// Caching middleware that serves responses from cache if available.
pub struct CacheMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for CacheMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

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

                let mut builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(flags)
                    .with_response(response_code)
                    .with_questions(message.questions().to_vec())
                    .with_authority_records(vec![result.soa_record]);

                if let Some(edns) = message.edns() {
                    let mut response_edns = Edns::default();
                    response_edns.set_do_bit(edns.do_bit());
                    builder = builder.with_edns(response_edns);
                }

                let bytes = builder.build().encode()?;

                Ok(Some(bytes))
            }

            CacheResult::Positive { records, ttl } => {
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

                let answers: Vec<_> = records
                    .iter()
                    .cloned()
                    .map(|mut r| {
                        r.ttl = ttl;
                        r
                    })
                    .collect();

                let mut builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_flags(flags)
                    .with_questions(message.questions().to_vec())
                    .with_answers(answers);

                if let Some(edns) = message.edns() {
                    let mut response_edns = Edns::default();
                    response_edns.set_do_bit(edns.do_bit());
                    builder = builder.with_edns(response_edns);
                }

                let bytes = builder.build().encode()?;

                Ok(Some(bytes))
            }
            CacheResult::Miss => Ok(None),
        }
    }
}
