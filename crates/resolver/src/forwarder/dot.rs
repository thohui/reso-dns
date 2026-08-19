use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};

use rustls::{ClientConfig, pki_types::ServerName};
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

use super::upstream::UpstreamError;

const ALPN_DOT: &[u8] = b"dot";

fn client_config() -> Result<Arc<ClientConfig>, UpstreamError> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

    if let Some(config) = CONFIG.get() {
        return Ok(config.clone());
    }

    let created_config = build_client_config().map_err(UpstreamError::Tls)?;

    Ok(CONFIG.get_or_init(|| created_config).clone())
}

fn build_client_config() -> Result<Arc<ClientConfig>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());

    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("tls protocol versions: {e}"))?
        .with_platform_verifier()
        .map_err(|e| format!("platform certificate verifier: {e}"))?
        .with_no_client_auth();

    config.alpn_protocols = vec![ALPN_DOT.to_vec()];

    Ok(Arc::new(config))
}

pub(crate) struct TlsUpstream {
    connector: TlsConnector,
    /// Identity the upstream certificate must match.
    server_name: ServerName<'static>,
}

/// The identity an upstream certificate must match.
///
/// `hostname` is the name the certificate is validated against.
/// This is an identity and not an adress, when absent the endpoint IP is used instead which
/// only works for servers whose certs carry a matching IP SAN
pub(crate) fn server_name(addr: SocketAddr, hostname: Option<&str>) -> Result<ServerName<'static>, UpstreamError> {
    let server_name = match hostname {
        Some(h) => ServerName::try_from(h)
            .map_err(|_| UpstreamError::Tls(format!("invalid tls hostname {h:?}")))?
            .to_owned(),
        // rustls omits SNI for IP identities, as RFC 6066 forbids IP literals there.
        None => ServerName::IpAddress(addr.ip().into()),
    };

    Ok(server_name)
}

impl TlsUpstream {
    pub fn new(addr: SocketAddr, hostname: Option<&str>) -> Result<Self, UpstreamError> {
        let server_name = server_name(addr, hostname)?;

        Ok(Self {
            connector: TlsConnector::from(client_config()?),
            server_name,
        })
    }

    pub async fn handshake(&self, sock: TcpStream) -> Result<TlsStream<TcpStream>, UpstreamError> {
        self.connector
            .connect(self.server_name.clone(), sock)
            .await
            .map_err(|e| UpstreamError::Tls(format!("handshake as {:?} failed: {e}", self.server_name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_identity_is_used_verbatim() {
        let upstream = TlsUpstream::new("9.9.9.9:853".parse().unwrap(), Some("dns.quad9.net")).unwrap();
        assert_eq!(upstream.server_name, ServerName::try_from("dns.quad9.net").unwrap());
    }

    #[test]
    fn falls_back_to_ip_identity() {
        let upstream = TlsUpstream::new("1.1.1.1:853".parse().unwrap(), None).unwrap();
        assert!(matches!(upstream.server_name, ServerName::IpAddress(_)));
    }

    #[test]
    fn hostname_that_is_an_ip_becomes_an_ip_identity() {
        let upstream = TlsUpstream::new("1.1.1.1:853".parse().unwrap(), Some("1.1.1.1")).unwrap();
        assert!(matches!(upstream.server_name, ServerName::IpAddress(_)));
    }

    #[test]
    fn rejects_invalid_hostname() {
        assert!(TlsUpstream::new("9.9.9.9:853".parse().unwrap(), Some("not a hostname")).is_err());
    }

    #[tokio::test]
    #[ignore = "requires external DoT connectivity"]
    async fn rejects_wrong_identity() {
        let addr: SocketAddr = "1.1.1.1:853".parse().unwrap(); // Cloudflare
        let upstream = TlsUpstream::new(addr, Some("dns.quad9.net")).unwrap();
        let sock = TcpStream::connect(addr).await.unwrap();
        assert!(upstream.handshake(sock).await.is_err());
    }
}
