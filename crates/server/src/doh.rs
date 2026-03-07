use std::{fs, io};

use std::{net::SocketAddr, sync::Arc};

use arc_swap::ArcSwap;
use base64::{Engine, engine::GeneralPurpose};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http2;
use hyper::{Method, Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use reso_context::{DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::{ServerError, ServerState, handle_request};

type Req = Request<Incoming>;
type Res = Response<Full<Bytes>>;

pub static BASE64_ENGINE: GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[derive(Clone)]
// An Executor that uses the tokio runtime.
pub struct TokioExecutor;

// Implement the `hyper::rt::Executor` trait for `TokioExecutor` so that it can be used to spawn
// tasks in the hyper runtime.
// An Executor allows us to manage execution of tasks which can help us improve the efficiency and
// scalability of the server.
impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DohConfig {
    /// Port to listen on for DoH requests.
    pub port: u16,
    /// Path to the TLS certificate file in PEM format.
    pub cert_path: String,
    /// Path to the TLS private key file in PEM format.
    pub key_path: String,
}

/// Run the DNS server over DoH.
#[allow(clippy::too_many_arguments)]
pub async fn run_doh<G, L>(
    config: DohConfig,
    bind_addr: SocketAddr,
    state: Arc<ArcSwap<ServerState<G, L>>>,
) -> anyhow::Result<()>
where
    G: Send + Sync + 'static,
    L: Send + Sync + Default + 'static,
{
    let _ = rustls::crypto::ring::default_provider().install_default();

    let certs = load_certs(&config.cert_path)?;
    let key = load_private_key(&config.key_path)?;

    let addr = SocketAddr::from((bind_addr.ip(), config.port));
    let listener = TcpListener::bind(addr).await?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| error(e.to_string()))?;

    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    tracing::info!("DOH listening on {}", addr);

    loop {
        let acceptor = tls_acceptor.clone();
        let (stream, client) = listener.accept().await?;

        let tls_stream = match acceptor.accept(stream).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("TLS accept error: {e}");
                continue;
            }
        };

        // check if the negotiated protocol is http 2
        let http2 = tls_stream.get_ref().1.alpn_protocol() == Some(b"h2");

        let io = TokioIo::new(tls_stream);

        let state = state.load_full();

        tokio::task::spawn(async move {
            let svc = service_fn(move |req: Req| handle_req(req, client, state.clone()));

            if http2 {
                // HTTP/2
                if let Err(e) = http2::Builder::new(TokioExecutor).serve_connection(io, svc).await {
                    tracing::error!("h2 conn error: {e}");
                }
            } else {
                // HTTP/1.1
                if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                    tracing::error!("h1 conn error: {e}");
                }
            }
        });
    }
}
async fn handle_req<G, L>(req: Req, addr: SocketAddr, state: Arc<ServerState<G, L>>) -> anyhow::Result<Res>
where
    G: Send + Sync + 'static,
    L: Send + Sync + Default + 'static,
{
    if req.uri().path() != "/dns-query" {
        return Ok(Response::builder().status(404).body(Full::new(Bytes::new()))?);
    }

    const MAX_RECV_SIZE: usize = 1232;

    let bytes = match *req.method() {
        Method::GET => match extract_bytes_from_get(req).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("failed to handle DOH GET request: {e:?}");
                return Ok(Response::builder().status(400).body(Full::new(Bytes::new()))?);
            }
        },
        Method::POST => match extract_bytes_from_post(req, MAX_RECV_SIZE).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("failed to handle DOH POST request: {e:?}");
                return Ok(Response::builder().status(400).body(Full::new(Bytes::new()))?);
            }
        },
        _ => {
            tracing::error!("unsupported DOH method: {}", req.method());
            return Ok(Response::builder().status(405).body(Full::new(Bytes::new()))?);
        }
    };

    let mut ctx = DnsRequestCtx::new(
        state.timeout,
        addr.ip(),
        RequestType::DOH,
        bytes,
        state.global.clone(),
        L::default(),
    );

    let response = handle_request(&mut ctx, state.clone()).await;

    match response {
        Ok(resp) => Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/dns-message")
            .body(Full::new(resp.bytes()))?),
        Err(e) => {
            let resp = match ctx.message() {
                Ok(m) => Response::builder()
                    .status(200)
                    .header("Content-Type", "application/dns-message")
                    .body(Full::new(create_error_message(m, &e)?))?,
                Err(_) => Response::builder().status(500).body(Full::new(Bytes::new()))?,
            };

            Ok(resp)
        }
    }
}

async fn extract_bytes_from_get(req: Req) -> anyhow::Result<Bytes> {
    let query_pairs = req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes())
            .into_owned()
            .collect::<Vec<(String, String)>>()
    });

    if let Some(pairs) = query_pairs {
        let doh_param = pairs.iter().find(|(k, _)| k == "dns");
        if let Some((_, v)) = doh_param {
            let decoded = BASE64_ENGINE.decode(v)?;
            return Ok(Bytes::from(decoded));
        }
    }

    Err(anyhow::anyhow!("no 'dns' query parameter found"))
}

async fn extract_bytes_from_post(req: Req, max_size: usize) -> anyhow::Result<Bytes> {
    use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};

    let content_type_ok = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("application/dns-message")
        })
        .unwrap_or(false);

    if !content_type_ok {
        let got = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        return Err(anyhow::anyhow!(
            "invalid content type: {}, expected application/dns-message",
            got
        ));
    }

    if let Some(len) = req.headers().get(CONTENT_LENGTH) {
        if let Ok(len) = len.to_str().unwrap_or("0").parse::<usize>() {
            if len > max_size {
                return Err(anyhow::anyhow!("request body too large: {}, max: {}", len, max_size));
            }
        } else {
            return Err(anyhow::anyhow!("invalid Content-Length header"));
        }
    }

    let bytes = req.collect().await?.to_bytes();
    if bytes.len() > max_size {
        return Err(anyhow::anyhow!(
            "request body too large after read: {}, max: {}",
            bytes.len(),
            max_size
        ));
    }
    Ok(bytes)
}

// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<CertificateDer<'static>>> {
    // Open certificate file.
    let certfile = fs::File::open(filename).map_err(|e| error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
fn load_private_key(filename: &str) -> anyhow::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile = fs::File::open(filename).map_err(|e| error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no private key found in {filename}"))
}

fn error(err: String) -> io::Error {
    io::Error::other(err)
}

fn create_error_message(message: &DnsMessage, error: &ServerError) -> anyhow::Result<Bytes> {
    let payload = DnsMessageBuilder::new()
        .with_id(message.id)
        .with_questions(message.questions().to_vec())
        .with_response(error.response_code())
        .build()
        .encode()?;
    Ok(payload)
}
