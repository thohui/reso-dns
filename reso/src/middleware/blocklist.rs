use async_trait::async_trait;
use bytes::Bytes;
use reso_context::{DnsMiddleware, DnsRequestCtx};
use reso_dns::{DnsMessage, DnsResponseCode};

use crate::{global::Global, local::Local};

pub struct BlocklistMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for BlocklistMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;
        let questions = message.questions();

        for question in questions {
            if ctx.global().blocklist.is_blocked(&question.qname) {
                let bytes = create_sinkhole_response(message).encode()?;
                return Ok(Some(bytes));
            }
        }

        Ok(None)
    }
}

fn create_sinkhole_response(msg: &DnsMessage) -> DnsMessage {
    let mut response = DnsMessage::new(
        msg.id,
        msg.flags,
        msg.questions().to_vec(),
        vec![],
        vec![],
        vec![],
    );

    response.flags.qr = true;
    response.flags.rcode_low = DnsResponseCode::NxDomain.into();

    response
}
