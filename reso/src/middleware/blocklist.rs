use async_trait::async_trait;
use bytes::Bytes;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsMessageBuilder, DnsResponseCode};

use crate::{global::Global, local::Local};

pub struct BlocklistMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for BlocklistMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;

        if let Some(question) = message.questions().first() {
            if ctx.global().blocklist.is_blocked(&question.qname) {
                let resp_bytes = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_questions(message.questions().to_vec())
                    .with_response(DnsResponseCode::NxDomain)
                    .build()
                    .encode()?;
                return Ok(Some(resp_bytes));
            }
        }

        Ok(None)
    }
}
