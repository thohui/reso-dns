use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsMessageBuilder, DnsResponseCode};

use crate::{global::Global, local::Local};

/// Middleware that blocks queries for blocklisted domain names.
pub struct BlocklistMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for BlocklistMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        let message = ctx.message()?;

        if let Some(question) = message.questions().first()
            && ctx.global().blocklist.is_blocked(&question.qname)
        {
            let resp_bytes = DnsMessageBuilder::new()
                .with_id(message.id)
                .with_questions(message.questions().to_vec())
                .with_response(DnsResponseCode::NxDomain)
                .build()
                .encode()?;

            ctx.local_mut().blocked = true;

            return Ok(Some(DnsResponse::from_bytes(resp_bytes)));
        }

        Ok(None)
    }
}
