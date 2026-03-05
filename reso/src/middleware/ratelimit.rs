use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsFlags, DnsMessage, DnsMessageBuilder, DnsOpcode, DnsResponseCode};

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
        false,
        false,
        query.flags.recursion_desired,
        true,
        false,
        query.flags.checking_disabled,
    )
}

#[async_trait]
impl DnsMiddleware<Global, Local> for RateLimitMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<bytes::Bytes>> {
        if self.limiter.check(ctx.request_address().ip()).await {
            Ok(None)
        } else {
            let message = ctx.message()?;
            ctx.local_mut().rate_limited = true;
            Ok(Some(
                echo_edns(
                    message,
                    DnsMessageBuilder::new()
                        .with_id(message.id)
                        .with_response(DnsResponseCode::Refused)
                        .with_flags(ratelimit_response_flags(message))
                        .with_questions(message.questions().to_vec()),
                )
                .build()
                .encode()?,
            ))
        }
    }
}
