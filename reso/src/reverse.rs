use std::{
    fmt::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use rand::RngExt;
use reso_dns::{
    ClassType, DnsMessage, DnsMessageBuilder, DnsQuestion, DnsResponseCode, RecordType,
    domain_name::{DomainName, unescape_label},
    message::DnsRecordData,
};
use tokio::{net::UdpSocket, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostnameCheck {
    Found(String),
    NotFound,
    Skipped,
    Failed,
}

pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            let cgnat = a == 100 && (64..128).contains(&b);
            v4.is_private() || v4.is_link_local() || cgnat
        }
        IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

fn reverse_name(ip: IpAddr) -> Option<DomainName> {
    let name = match ip.to_canonical() {
        IpAddr::V4(v4) => {
            let [a, b, c, d] = v4.octets();
            format!("{d}.{c}.{b}.{a}.in-addr.arpa")
        }
        IpAddr::V6(v6) => {
            let mut name = String::with_capacity(72);
            for byte in v6.octets().iter().rev() {
                let _ = write!(name, "{:x}.{:x}.", byte & 0xf, byte >> 4);
            }
            name.push_str("ip6.arpa");
            name
        }
    };
    DomainName::from_ascii(name).ok()
}

#[cfg(target_os = "linux")]
pub fn default_gateway() -> Option<IpAddr> {
    let route_table = std::fs::read_to_string("/proc/net/route").ok()?;
    route_table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [_, destination, gateway, flags, _, _, metric, mask, ..] = fields.as_slice() else {
                return None;
            };
            // 0x2 = RTF_GATEWAY, the mask check skips split routes like 0.0.0.0/1
            let flags = u16::from_str_radix(flags, 16).ok()?;
            if *destination != "00000000" || *mask != "00000000" || flags & 0x2 == 0 {
                return None;
            }
            let raw = u32::from_str_radix(gateway, 16).ok()?;
            let ip = Ipv4Addr::from(raw.to_ne_bytes());
            let metric: u32 = metric.parse().ok()?;
            (!ip.is_unspecified()).then_some((metric, IpAddr::V4(ip)))
        })
        .min_by_key(|(metric, _)| *metric)
        .map(|(_, ip)| ip)
}

#[cfg(not(target_os = "linux"))]
pub fn default_gateway() -> Option<IpAddr> {
    None
}

pub async fn lookup_ptr(servers: &[SocketAddr], ip: IpAddr, timeout: Duration) -> HostnameCheck {
    let Some(name) = reverse_name(ip) else {
        return HostnameCheck::NotFound;
    };

    let mut any_failed = servers.is_empty();
    for server in servers {
        match query_ptr(*server, &name, timeout).await {
            Ok(HostnameCheck::Found(hostname)) => return HostnameCheck::Found(hostname),
            Ok(HostnameCheck::NotFound | HostnameCheck::Skipped) => {}
            Ok(HostnameCheck::Failed) => any_failed = true,
            Err(e) => {
                tracing::debug!(%server, %ip, "ptr lookup failed: {e}");
                any_failed = true;
            }
        }
    }
    if any_failed {
        HostnameCheck::Failed
    } else {
        HostnameCheck::NotFound
    }
}

async fn query_ptr(server: SocketAddr, name: &DomainName, timeout: Duration) -> anyhow::Result<HostnameCheck> {
    let deadline = Instant::now() + timeout;
    let bind: SocketAddr = if server.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(bind).await?;
    socket.connect(server).await?;

    let id = rand::rng().random::<u16>();
    let question = DnsQuestion::new(name.clone(), RecordType::PTR, ClassType::IN);
    let query = DnsMessageBuilder::new()
        .with_id(id)
        .add_question(question.clone())
        .build()
        .encode()?;
    socket.send(&query).await?;

    let mut buf = [0u8; 4096];
    loop {
        let len = tokio::time::timeout_at(deadline, socket.recv(&mut buf)).await??;
        let Some(bytes) = buf.get(..len) else {
            continue;
        };
        let Ok(response) = DnsMessage::decode(bytes) else {
            continue;
        };
        if response.id != id || !response.flags.response || response.questions() != std::slice::from_ref(&question) {
            continue;
        }
        return Ok(ptr_result(&response, name));
    }
}

fn ptr_result(response: &DnsMessage, name: &DomainName) -> HostnameCheck {
    if response.flags.truncated && response.answers().is_empty() {
        return HostnameCheck::Failed;
    }

    match response.response_code() {
        DnsResponseCode::NoError => response
            .answers()
            .iter()
            .filter(|r| r.name == *name)
            .find_map(|r| match (r.record_type(), r.data()) {
                (RecordType::PTR, DnsRecordData::DomainName(target)) if !target.is_root() => {
                    let hostname = display_hostname(target);
                    let usable = hostname.split('.').next().is_some_and(|label| !label.trim().is_empty());
                    usable.then_some(HostnameCheck::Found(hostname))
                }
                _ => None,
            })
            .unwrap_or(HostnameCheck::NotFound),
        DnsResponseCode::NxDomain => HostnameCheck::NotFound,
        _ => HostnameCheck::Failed,
    }
}

/// Readable form of a PTR target, with escapes and punycode decoded.
fn display_hostname(name: &DomainName) -> String {
    // dots inside a label are escaped by as_str, so every `.` is a label boundary
    let labels: Vec<String> = name
        .as_str()
        .split('.')
        .map(|label| {
            let text = String::from_utf8_lossy(&unescape_label(label)).into_owned();
            let punycode = text.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("xn--"));
            if punycode {
                text.get(4..).and_then(idna::punycode::decode_to_string).unwrap_or(text)
            } else {
                text
            }
        })
        .collect();
    labels.join(".").chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> DomainName {
        DomainName::from_ascii(s).unwrap()
    }

    #[test]
    fn reverse_name_v4_and_v6() {
        assert_eq!(
            reverse_name("192.168.2.1".parse().unwrap()).unwrap(),
            name("1.2.168.192.in-addr.arpa")
        );
        assert_eq!(
            reverse_name("fd00::1".parse().unwrap()).unwrap(),
            name("1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.d.f.ip6.arpa")
        );
        assert_eq!(
            reverse_name("::ffff:10.0.0.5".parse().unwrap()).unwrap(),
            name("5.0.0.10.in-addr.arpa")
        );
    }

    #[test]
    fn private_ips() {
        assert!(is_private_ip("192.168.1.20".parse().unwrap()));
        assert!(is_private_ip("100.64.1.1".parse().unwrap()));
        assert!(!is_private_ip("100.128.0.1".parse().unwrap()));
        assert!(is_private_ip("fd12::1".parse().unwrap()));
        assert!(is_private_ip("::ffff:192.168.1.20".parse().unwrap()));
        assert!(!is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_private_ip("::1".parse().unwrap()));
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn ptr_answers() {
        let qname = name("5.2.168.192.in-addr.arpa");
        let question = DnsQuestion::new(qname.clone(), RecordType::PTR, ClassType::IN);
        let ptr = |owner: &str, target: &str| {
            reso_dns::DnsRecord::new(
                name(owner),
                RecordType::PTR,
                ClassType::IN,
                60,
                DnsRecordData::DomainName(name(target)),
            )
        };

        let found = DnsMessageBuilder::new()
            .add_question(question.clone())
            .add_answer(ptr("9.9.168.192.in-addr.arpa", "other.lan"))
            .add_answer(ptr("5.2.168.192.in-addr.arpa", "laptop.lan"))
            .with_response(DnsResponseCode::NoError)
            .build();
        assert_eq!(ptr_result(&found, &qname), HostnameCheck::Found("laptop.lan".into()));

        let unrelated = DnsMessageBuilder::new()
            .add_question(question.clone())
            .add_answer(ptr("9.9.168.192.in-addr.arpa", "other.lan"))
            .with_response(DnsResponseCode::NoError)
            .build();
        assert_eq!(ptr_result(&unrelated, &qname), HostnameCheck::NotFound);

        let nx = DnsMessageBuilder::new()
            .add_question(question.clone())
            .with_response(DnsResponseCode::NxDomain)
            .build();
        assert_eq!(ptr_result(&nx, &qname), HostnameCheck::NotFound);

        let servfail = DnsMessageBuilder::new()
            .add_question(question)
            .with_response(DnsResponseCode::ServerFailure)
            .build();
        assert_eq!(ptr_result(&servfail, &qname), HostnameCheck::Failed);
    }

    #[test]
    fn hostname_display() {
        let display = |s: &str| display_hostname(&name(s));
        assert_eq!(display("b\\195\\188ro-pc.lan"), "büro-pc.lan");
        assert_eq!(display("johns\\032iphone.lan"), "johns iphone.lan");
        assert_eq!(display("a\\010b.lan"), "ab.lan");
        assert_eq!(display("laptop.lan"), "laptop.lan");

        let punycode = idna::domain_to_ascii("büro-pc.lan").unwrap();
        assert!(punycode.starts_with("xn--"));
        assert_eq!(display(&punycode), "büro-pc.lan");
    }
}
