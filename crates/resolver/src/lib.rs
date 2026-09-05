use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use reso_context::{DnsProtocol, DnsRequestCtx, DnsResponse, ErrorType};
use reso_dns::DnsResponseCode;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Trait for DNS resolvers that can resolve DNS requests.
#[async_trait]
pub trait DnsResolver<G: Send + Sync, L> {
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> Result<DnsResponse, ResolveError>;
}
/// DynResolver
pub type DynResolver<G, L> = dyn DnsResolver<G, L> + Send + Sync;

#[derive(Error, Debug)]
#[error("{kind}")]
pub struct ResolveError {
    #[source]
    kind: ResolveErrorKind,
    protocol: Option<DnsProtocol>,
}

impl ResolveError {
    pub fn kind(&self) -> &ResolveErrorKind {
        &self.kind
    }

    /// Protocol of the upstream attempt this error came from, if one was made.
    pub fn protocol(&self) -> Option<DnsProtocol> {
        self.protocol
    }

    pub fn response_code(&self) -> DnsResponseCode {
        self.kind.response_code()
    }

    pub fn error_type(&self) -> ErrorType {
        self.kind.error_type()
    }
}

impl From<ResolveErrorKind> for ResolveError {
    fn from(kind: ResolveErrorKind) -> Self {
        Self { kind, protocol: None }
    }
}

/// Error type for DNS resolvers
#[derive(Error, Debug)]
pub enum ResolveErrorKind {
    #[error("request timed out")]
    Timeout,

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("malformed response: {0}")]
    MalformedResponse(String),

    #[error("{0}")]
    Other(String),
}

impl ResolveErrorKind {
    pub fn with_protocol(self, protocol: Option<DnsProtocol>) -> ResolveError {
        ResolveError { kind: self, protocol }
    }

    pub fn response_code(&self) -> DnsResponseCode {
        match self {
            ResolveErrorKind::Timeout => DnsResponseCode::ServerFailure,
            ResolveErrorKind::InvalidRequest(_) => DnsResponseCode::Refused,
            ResolveErrorKind::InvalidResponse(_) => DnsResponseCode::ServerFailure,
            ResolveErrorKind::MalformedResponse(_) => DnsResponseCode::ServerFailure,
            ResolveErrorKind::Other(_) => DnsResponseCode::ServerFailure,
        }
    }

    pub fn error_type(&self) -> ErrorType {
        match self {
            Self::Timeout => ErrorType::Timeout,
            Self::InvalidRequest(_) => ErrorType::InvalidRequest,
            Self::InvalidResponse(_) => ErrorType::InvalidResponse,
            Self::MalformedResponse(_) => ErrorType::MalformedResponse,
            Self::Other(_) => ErrorType::Other,
        }
    }
}

/// Default port for plain DNS.
const DEFAULT_PLAIN_PORT: u16 = 53;
/// Default port for DNS over TLS (RFC 7858).
const DEFAULT_TLS_PORT: u16 = 853;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "lowercase")]
pub enum Upstream {
    /// UDP and TCP
    Plain {
        #[serde(deserialize_with = "deserialize_plain_endpoint")]
        endpoint: SocketAddr,
    },
    /// DNS over TLS
    Tls {
        #[serde(deserialize_with = "deserialize_tls_endpoint")]
        endpoint: SocketAddr,
        /// Name the upstream certificate is validated against, and sent as SNI.
        /// This is an identity and not an adress, when absent the endpoint IP is used instead which
        /// only works for servers whose certs carry a matching IP SAN
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hostname: Option<String>,
    },
    // DNS over Https
    // Doh { url: Url },
}

impl Upstream {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Plain { .. } => Ok(()),
            Self::Tls { endpoint, hostname } => forwarder::dot::server_name(*endpoint, hostname.as_deref())
                .map(|_| ())
                .map_err(|e| e.to_string()),
        }
    }
}

fn deserialize_endpoint<'de, D>(deserializer: D, default_port: u16) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(addr);
    }

    s.parse::<IpAddr>()
        .map(|ip| SocketAddr::new(ip, default_port))
        .map_err(|_| serde::de::Error::custom(format!("invalid endpoint {s:?}: expected `IP` or `IP:port`")))
}

fn deserialize_plain_endpoint<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SocketAddr, D::Error> {
    deserialize_endpoint(deserializer, DEFAULT_PLAIN_PORT)
}

fn deserialize_tls_endpoint<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SocketAddr, D::Error> {
    deserialize_endpoint(deserializer, DEFAULT_TLS_PORT)
}

pub mod forwarder;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Upstream {
        serde_json::from_str(json).expect("valid upstream")
    }

    #[test]
    fn plain_endpoint_defaults_to_53() {
        let Upstream::Plain { endpoint } = parse(r#"{"kind":"plain","endpoint":"1.1.1.1"}"#) else {
            panic!("expected plain upstream");
        };
        assert_eq!(endpoint, "1.1.1.1:53".parse().unwrap());
    }

    #[test]
    fn tls_endpoint_defaults_to_853() {
        let Upstream::Tls { endpoint, hostname } = parse(r#"{"kind":"tls","endpoint":"9.9.9.9"}"#) else {
            panic!("expected tls upstream");
        };
        assert_eq!(endpoint, "9.9.9.9:853".parse().unwrap());
        assert_eq!(hostname, None);
    }

    #[test]
    fn explicit_port_is_kept() {
        let Upstream::Plain { endpoint } = parse(r#"{"kind":"plain","endpoint":"1.1.1.1:5353"}"#) else {
            panic!("expected plain upstream");
        };
        assert_eq!(endpoint, "1.1.1.1:5353".parse().unwrap());
    }

    #[test]
    fn ipv6_with_and_without_port() {
        let Upstream::Tls { endpoint, .. } = parse(r#"{"kind":"tls","endpoint":"2620:fe::fe"}"#) else {
            panic!("expected tls upstream");
        };
        assert_eq!(endpoint, "[2620:fe::fe]:853".parse().unwrap());

        let Upstream::Tls { endpoint, .. } = parse(r#"{"kind":"tls","endpoint":"[2620:fe::fe]:8853"}"#) else {
            panic!("expected tls upstream");
        };
        assert_eq!(endpoint, "[2620:fe::fe]:8853".parse().unwrap());
    }

    #[test]
    fn hostname_is_optional_and_preserved() {
        let Upstream::Tls { hostname, .. } = parse(r#"{"kind":"tls","endpoint":"9.9.9.9","hostname":"dns.quad9.net"}"#)
        else {
            panic!("expected tls upstream");
        };
        assert_eq!(hostname.as_deref(), Some("dns.quad9.net"));
    }

    #[test]
    fn rejects_garbage_endpoint() {
        assert!(serde_json::from_str::<Upstream>(r#"{"kind":"tls","endpoint":"dns.quad9.net"}"#).is_err());
    }

    #[test]
    fn validate_rejects_bad_tls_hostname() {
        let upstream = parse(r#"{"kind":"tls","endpoint":"9.9.9.9","hostname":"dns.quad9 .net"}"#);
        assert!(upstream.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_upstreams() {
        let quad9 = parse(r#"{"kind":"tls","endpoint":"9.9.9.9","hostname":"dns.quad9.net"}"#);
        assert!(quad9.validate().is_ok());
        assert!(parse(r#"{"kind":"tls","endpoint":"1.1.1.1"}"#).validate().is_ok());
        assert!(parse(r#"{"kind":"plain","endpoint":"1.1.1.1"}"#).validate().is_ok());
    }

    #[test]
    fn round_trips_through_json() {
        let original = r#"{"kind":"tls","endpoint":"9.9.9.9:853","hostname":"dns.quad9.net"}"#;
        let encoded = serde_json::to_string(&parse(original)).unwrap();
        assert_eq!(encoded, original);
    }

    #[test]
    fn omitted_hostname_is_not_serialized() {
        let encoded = serde_json::to_string(&parse(r#"{"kind":"tls","endpoint":"9.9.9.9"}"#)).unwrap();
        assert_eq!(encoded, r#"{"kind":"tls","endpoint":"9.9.9.9:853"}"#);
    }
}
