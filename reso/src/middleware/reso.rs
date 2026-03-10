use std::{
    net::{Ipv4Addr, Ipv6Addr, UdpSocket},
    sync::LazyLock,
};

use async_trait::async_trait;
use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};
use reso_dns::{
    ClassType, DnsFlags, DnsMessageBuilder, DnsOpcode, DnsRecord, DnsResponseCode, RecordType, domain_name::DomainName,
    message::DnsRecordData,
};

use crate::{global::Global, local::Local, middleware::echo_edns};

static RESO_LOCAL: LazyLock<DomainName> = LazyLock::new(|| DomainName::from_ascii("reso.dns").unwrap());
const TTL: u32 = 60;

pub struct ResoLocalMiddleware {
    a: Option<DnsRecord>,
    aaaa: Option<DnsRecord>,
}

impl ResoLocalMiddleware {
    pub fn new() -> Self {
        let a = local_ipv4().map(|ip| {
            DnsRecord::new(
                RESO_LOCAL.clone(),
                RecordType::A,
                ClassType::IN,
                TTL,
                DnsRecordData::Ipv4(ip),
            )
        });
        let aaaa = local_ipv6().map(|ip| {
            DnsRecord::new(
                RESO_LOCAL.clone(),
                RecordType::AAAA,
                ClassType::IN,
                TTL,
                DnsRecordData::Ipv6(ip),
            )
        });

        Self { a, aaaa }
    }
}

#[async_trait]
impl DnsMiddleware<Global, Local> for ResoLocalMiddleware {
    async fn on_query(&self, ctx: &mut DnsRequestCtx<Global, Local>) -> anyhow::Result<Option<DnsResponse>> {
        let message = ctx.message()?;
        let question = match message.questions().first() {
            Some(q) => q,
            None => return Ok(None),
        };

        if question.qname != *RESO_LOCAL {
            return Ok(None);
        }

        let answers: Vec<DnsRecord> = match question.qtype {
            RecordType::A => self.a.iter().cloned().collect(),
            RecordType::AAAA => self.aaaa.iter().cloned().collect(),
            RecordType::ANY => self.a.iter().chain(self.aaaa.iter()).cloned().collect(),
            _ => return Ok(None),
        };

        let flags = DnsFlags::new(
            true,
            DnsOpcode::Query,
            false,
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

        Ok(Some(DnsResponse::from_bytes(bytes)))
    }
}

fn local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}

fn local_ipv6() -> Option<Ipv6Addr> {
    let socket = UdpSocket::bind("[::]:0").ok()?;
    socket.connect("[2001:4860:4860::8888]:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V6(ip) => Some(ip),
        _ => None,
    }
}
