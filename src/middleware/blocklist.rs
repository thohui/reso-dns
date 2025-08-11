use async_trait::async_trait;
use bytes::Bytes;

use crate::{
    dns::message::{DnsMessage, DnsResponseCode},
    resolver::DnsRequestCtx,
};

use super::DnsMiddleware;

pub struct BlocklistMiddleware;

#[async_trait]
impl DnsMiddleware for BlocklistMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>> {
        let message = ctx.message()?;
        let questions = message.questions();

        for question in questions {
            if ctx.services().blocklist.is_blocked(&question.qname) {
                tracing::debug!("blocked {}", question.qname);
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
    let response_code: u16 = DnsResponseCode::NxDomain.into();
    response.flags.rcode_low = response_code as u8;

    response
}
