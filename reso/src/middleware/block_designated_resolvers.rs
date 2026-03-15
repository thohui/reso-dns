use std::sync::LazyLock;

use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{DnsMessageBuilder, DnsResponseCode, RecordType, domain_name::DomainName};

use crate::{global::Global, local::Local, middleware::echo_edns};

pub struct BlockDesignatedResolversMiddleware;

static ICLOUD_RELAY_DOMAINS: LazyLock<Vec<DomainName>> = LazyLock::new(|| {
    vec![
        DomainName::from_ascii("mask.icloud.com").unwrap(),
        DomainName::from_ascii("mask-h2.icloud.com").unwrap(),
    ]
});

static DESIGNATED_RESOLVER_ZONE: LazyLock<DomainName> =
    LazyLock::new(|| DomainName::from_ascii("resolver.arpa").unwrap());

static FIREFOX_CANARY_DOMAIN: LazyLock<DomainName> =
    LazyLock::new(|| DomainName::from_ascii("use-application-dns.net").unwrap());

#[async_trait::async_trait]
impl DnsMiddleware<Global, Local> for BlockDesignatedResolversMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        let message = ctx.message()?;
        let questions = message.questions();

        let config = ctx.global().config.get_config();

        let should_process = config.dns.security.block_icloud_private_relay
            || config.dns.security.block_designated_resolver
            || config.dns.security.block_firefox_canary;

        if !should_process {
            return Ok(None);
        }

        for question in questions {
            let qname = &question.qname;

            // iCloud Private Relay
            if config.dns.security.block_icloud_private_relay
                && ICLOUD_RELAY_DOMAINS
                    .iter()
                    .any(|d| d == qname && matches!(question.qtype, RecordType::A | RecordType::AAAA))
            {
                let builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_questions(questions.to_vec())
                    .with_response(DnsResponseCode::NxDomain);

                let response_message = echo_edns(message, builder).build();
                let bytes = response_message.encode()?;

                tracing::debug!("blocked Apple Private Relay query for {}", qname);
                return Ok(Some(DnsResponse::from_parsed(bytes, response_message)));
            }

            // Firefox Canary
            if config.dns.security.block_firefox_canary && qname == &*FIREFOX_CANARY_DOMAIN {
                let builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_questions(questions.to_vec())
                    .with_response(DnsResponseCode::NxDomain);

                let response_message = echo_edns(message, builder).build();
                let bytes = response_message.encode()?;

                tracing::debug!("Blocked Firefox Canary query for {}", qname);
                return Ok(Some(DnsResponse::from_parsed(bytes, response_message)));
            }

            // Designated Resolver
            if config.dns.security.block_designated_resolver
                && (qname == &*DESIGNATED_RESOLVER_ZONE
                    || qname.ends_with(&format!(".{}", DESIGNATED_RESOLVER_ZONE.as_str())))
            {
                let builder = DnsMessageBuilder::new()
                    .with_id(message.id)
                    .with_questions(questions.to_vec())
                    .with_response(DnsResponseCode::NoError);

                let response_message = echo_edns(message, builder).build();
                let bytes = response_message.encode()?;
                tracing::info!("Blocked Designated Resolver query for {}", qname);
                return Ok(Some(DnsResponse::from_parsed(bytes, response_message)));
            }
        }

        Ok(None)
    }
}
