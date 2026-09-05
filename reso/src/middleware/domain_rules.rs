use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsFlags, DnsMessageBuilder, DnsResponseCode};

use crate::{global::Global, local::Local, services::domain_rules::RuleMatch};

/// Middleware that blocks queries for blocked domain names.
pub struct DomainRulesMiddleware;

#[async_trait]
impl DnsMiddleware<Global, Local> for DomainRulesMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        let message = ctx.message()?;

        let Some(question) = message.questions().first() else {
            return Ok(None);
        };

        let rule = ctx.global().domain_rules.match_rule(&question.qname);

        if let RuleMatch::Blocked(_) = rule {
            let flags = DnsFlags::new(
                true,
                message.flags.opcode,
                false,
                false,
                message.flags.recursion_desired,
                true,
                false,
                message.flags.checking_disabled,
            );

            let message = DnsMessageBuilder::new()
                .with_id(message.id)
                .with_flags(flags)
                .with_questions(message.questions().to_vec())
                .with_response(DnsResponseCode::NxDomain)
                .build();

            let bytes = message.encode()?;

            ctx.local_mut().blocked = true;

            return Ok(Some(DnsResponse::from_parsed(bytes, message, None)));
        }

        ctx.local_mut().rule_id = rule.id();

        Ok(None)
    }
}
