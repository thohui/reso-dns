use base64::{Engine, engine::general_purpose::STANDARD};
use rand::Rng;
use std::{
    env::{self, VarError},
    fs,
    net::SocketAddr,
    path::Path,
    str::FromStr,
};
use tracing::Level;

const DEFAULT_DATABASE_PATH: &str = "reso.db";
const DEFAULT_METRICS_DATABASE_PATH: &str = "reso_metrics.db";
const DEFAULT_SESSION_SECRET_PATH: &str = "reso_session.key";

pub struct EnvConfig {
    pub log_level: Level,
    pub db_path: String,
    pub metrics_db_path: String,
    pub dns_server_address: SocketAddr,
    pub http_server_address: SocketAddr,
    pub cookie_secret: Vec<u8>,
}

impl EnvConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let log_level = match env::var("RESO_LOG_LEVEL") {
            Ok(level) => Level::from_str(&level)?,
            Err(_) => Level::INFO,
        };
        let db_path = env::var("RESO_DATABASE_PATH").unwrap_or(DEFAULT_DATABASE_PATH.to_owned());
        let metrics_db_path =
            env::var("RESO_METRICS_DATABASE_PATH").unwrap_or(DEFAULT_METRICS_DATABASE_PATH.to_owned());

        if Path::new(&db_path) == Path::new(&metrics_db_path) {
            anyhow::bail!("RESO_DATABASE_PATH cannot point to the same path as RESO_METRICS_DATABASE_PATH")
        }

        let dns_server_address = env::var("RESO_DNS_SERVER_ADDRESS").unwrap_or("127.0.0.1:53".to_owned());
        let http_server_address = env::var("RESO_HTTP_SERVER_ADDRESS").unwrap_or("127.0.0.1:80".to_owned());

        let session_secret_path =
            env::var("RESO_SESSION_SECRET_PATH").unwrap_or(DEFAULT_SESSION_SECRET_PATH.to_owned());

        // backwards compatibility: in older versions of reso the secret was stored in the
        // RESO_COOKIE_SECRET environment variable that the user had to supply.
        // in newer versions reso generates the secret itself and stores it in a file.
        if !Path::new(&session_secret_path).exists() {
            // write legacy RESO_COOKIE_SECRET to disk.
            if let Ok(val) = env::var("RESO_COOKIE_SECRET") {
                let decoded = STANDARD.decode(&val)?;
                if decoded.len() != 32 {
                    anyhow::bail!(
                        "RESO_COOKIE_SECRET must decode to exactly 32 bytes, got {}",
                        decoded.len()
                    );
                }
                let secret: [u8; 32] = decoded.try_into().unwrap();
                fs::write(&session_secret_path, secret)?;
            }
        }

        let cookie_secret: Vec<u8> = load_or_create_session_secret(&session_secret_path)?.into();

        Ok(Self {
            log_level,
            db_path,
            metrics_db_path,
            dns_server_address: SocketAddr::from_str(&dns_server_address)?,
            http_server_address: SocketAddr::from_str(&http_server_address)?,
            cookie_secret,
        })
    }
}

fn load_or_create_session_secret(path: &str) -> anyhow::Result<[u8; 32]> {
    let path = Path::new(path);
    if path.exists() {
        let bytes = fs::read(path)?;
        return bytes.try_into().map_err(|_| {
            anyhow::anyhow!(
                "Session secret file at '{}' has invalid length, expected 32 bytes",
                path.display()
            )
        });
    }
    let secret: [u8; 32] = rand::rng().random();
    fs::write(path, secret)?;
    Ok(secret)
}
