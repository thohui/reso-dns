use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsFlags, DnsMessageBuilder, DnsResponseCode};

use crate::{
    global::Global,
    local::Local,
    services::domain_rules::RuleMatch::{self},
};

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

        match rule {
            RuleMatch::Blocked(id) => {
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
                ctx.local_mut().rule_id = Some(id);

                return Ok(Some(DnsResponse::from_parsed(bytes, message, None)));
            }
            RuleMatch::Allowed(rule_id) => {
                ctx.local_mut().rule_id = Some(rule_id);
                Ok(None)
            }
            RuleMatch::NoMatch => Ok(None),
        }
    }
}
