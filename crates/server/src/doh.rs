use std::{fs, io};

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use base64::{Engine, engine::GeneralPurpose};
use bytes::{BufMut, Bytes, BytesMut};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http2;
use hyper::{Method, Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use reso_context::{DnsMiddleware, DnsRequestCtx, RequestType};
use reso_dns::{DnsMessage, DnsMessageBuilder, DnsResponseCode};
use reso_resolver::DnsResolver;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::{DohConfig, ErrorCallback, SuccessCallback};

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

/// Run the DNS server over DoH.
#[allow(clippy::too_many_arguments)]
pub async fn run_doh<L, G, R>(
    config: DohConfig,
    bind_addr: SocketAddr,
    resolver: Arc<R>,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    global: Arc<G>,
    recv_size: usize,
    timeout: Duration,
    on_success: Option<SuccessCallback<G, L>>,
    on_error: Option<ErrorCallback<G, L>>,
) -> anyhow::Result<()>
where
    R: DnsResolver<G, L> + Send + Sync + 'static,
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
        let (stream, _) = listener.accept().await?;

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

        let resolver = resolver.clone();
        let global = global.clone();

        let middlewares = middlewares.clone();

        let on_success = on_success.clone();
        let on_error = on_error.clone();

        tokio::task::spawn(async move {
            let svc = service_fn(move |req: Req| {
                handle_req(
                    resolver.clone(),
                    global.clone(),
                    timeout,
                    req,
                    recv_size,
                    middlewares.clone(),
                    on_success.clone(),
                    on_error.clone(),
                )
            });

            if http2 {
                // HTTP/2
                if let Err(e) = http2::Builder::new(TokioExecutor)
                    .serve_connection(io, svc)
                    .await
                {
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
#[allow(clippy::too_many_arguments)]
async fn handle_req<G, L, R>(
    resolver: Arc<R>,
    global: Arc<G>,
    timeout: Duration,
    req: Req,
    max_size: usize,
    middlewares: Arc<Vec<Arc<dyn DnsMiddleware<G, L> + 'static>>>,
    on_success: Option<SuccessCallback<G, L>>,
    on_error: Option<ErrorCallback<G, L>>,
) -> anyhow::Result<Res>
where
    R: DnsResolver<G, L> + Send + Sync + 'static,
    G: Send + Sync + 'static,
    L: Send + Sync + Default + 'static,
{
    if req.uri().path() != "/dns-query" {
        return Ok(Response::builder()
            .status(404)
            .body(Full::new(Bytes::new()))?);
    }

    let bytes = match *req.method() {
        Method::GET => match extract_bytes_from_get(req).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("failed to handle DOH GET request: {e:?}");
                return Ok(Response::builder()
                    .status(400)
                    .body(Full::new(Bytes::new()))?);
            }
        },
        Method::POST => match extract_bytes_from_post(req, max_size).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("failed to handle DOH POST request: {e:?}");
                return Ok(Response::builder()
                    .status(400)
                    .body(Full::new(Bytes::new()))?);
            }
        },
        _ => {
            tracing::error!("unsupported DOH method: {}", req.method());
            return Ok(Response::builder()
                .status(405)
                .body(Full::new(Bytes::new()))?);
        }
    };

    let ctx = DnsRequestCtx::new(timeout, RequestType::DOH, bytes, global, L::default());

    if let Ok(Some(bytes)) = reso_context::run_middlewares(middlewares, &ctx).await {
        let resp = Response::builder()
            .status(200)
            .header("Content-Type", "application/dns-message")
            .body(Full::new(bytes.clone()))?;

        tokio::spawn(async move {
            if let Some(on_success) = on_success {
                let _ = on_success(&ctx, &bytes).await;
            }
        });

        return Ok(resp);
    }

    match resolver.resolve(&ctx).await {
        Ok(b) => {
            let resp = Response::builder()
                .status(200)
                .header("Content-Type", "application/dns-message")
                .body(Full::new(b.clone()))?;

            tokio::spawn(async move {
                if let Some(on_success) = on_success {
                    let _ = on_success(&ctx, &b).await;
                }
            });

            Ok(resp)
        }
        Err(e) => {
            let message = ctx.message()?;
            let resp_bytes = create_server_error_message(message)?;
            let resp = Response::builder()
                .status(502)
                .body(Full::new(resp_bytes))?;
            tokio::spawn(async move {
                if let Some(on_error) = on_error {
                    let _ = on_error(&ctx, &e).await;
                }
            });
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

    // Be tolerant: case-insensitive, ignore parameters.
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
                return Err(anyhow::anyhow!(
                    "request body too large: {}, max: {}",
                    len,
                    max_size
                ));
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
    let certfile =
        fs::File::open(filename).map_err(|e| error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile =
        fs::File::open(filename).map_err(|e| error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

/// Create a DNS server failure message with the given transaction ID.
fn create_server_error_message(message: &DnsMessage) -> anyhow::Result<Bytes> {
    let payload = DnsMessageBuilder::new()
        .with_id(message.id)
        .with_questions(message.questions().to_vec())
        .with_response(DnsResponseCode::ServerFailure)
        .build()
        .encode()?;

    let len = u16::try_from(payload.len()).context("DNS payload exceeds 65535 bytes")?;
    let mut resp = BytesMut::with_capacity(2 + payload.len());

    resp.put_u16(len);
    resp.extend_from_slice(&payload);

    Ok(resp.freeze())
}
