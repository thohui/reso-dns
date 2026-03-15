use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsFlags, DnsMessage, DnsMessageBuilder, DnsResponseCode};

use crate::{
    global::Global,
    local::Local,
    middleware::echo_edns,
    ratelimit::{RateLimitConfig, RateLimiter},
};

pub struct RateLimitMiddleware {
    limiter: RateLimiter,
}

impl RateLimitMiddleware {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: RateLimiter::new(config),
        }
    }
}

fn ratelimit_response_flags(query: &DnsMessage) -> DnsFlags {
    DnsFlags::new(
        true,
        query.flags.opcode,
        true,
        false,
        query.flags.recursion_desired,
        true,
        false,
        query.flags.checking_disabled,
    )
}

#[async_trait]
impl DnsMiddleware<Global, Local> for RateLimitMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        if self.limiter.check(ctx.request_address().clone()).await {
            Ok(None)
        } else {
            ctx.local_mut().rate_limited = true;
            let message = ctx.message()?;
            let message = echo_edns(
                message,
                DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_response(DnsResponseCode::Refused)
                    .with_flags(ratelimit_response_flags(message))
                    .with_questions(message.questions().to_vec()),
            )
            .build();

            let bytes = message.encode()?;
            Ok(Some(DnsResponse::from_parsed(bytes, message)))
        }
    }
}
