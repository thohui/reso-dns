use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsFlags, DnsMessageBuilder, DnsOpcode, DnsResponseCode};

use crate::{global::Global, local::Local, middleware::echo_edns, services::local_records::LocalLookup};

pub struct LocalRecordsMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for LocalRecordsMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        let message = ctx.message()?;
        let question = match message.questions().first() {
            Some(q) => q,
            None => return Ok(None),
        };

        // NODATA keeps address queries for local names from going upstream.
        let answers = match ctx.global().local_records.lookup(&question.qname, question.qtype) {
            LocalLookup::Answer(records) => records.into_iter().map(|r| r.record).collect(),
            LocalLookup::NoData => Vec::new(),
            LocalLookup::NotFound => return Ok(None),
        };

        let flags = DnsFlags::new(
            true,
            DnsOpcode::Query,
            true, // authoritative
            false,
            message.flags.recursion_desired,
            true,
            false,
            message.flags.checking_disabled,
        );

        let bytes = echo_edns(
            message,
            DnsMessageBuilder::new()
                .with_id(message.id)
                .with_flags(flags)
                .with_response(DnsResponseCode::NoError)
                .with_questions(message.questions().to_vec())
                .with_answers(answers),
        )
        .build()
        .encode()?;

        Ok(Some(DnsResponse::from_bytes(bytes, None)))
    }
}
