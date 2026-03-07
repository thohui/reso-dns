use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    env::{self, VarError},
    net::SocketAddr,
    str::FromStr,
};
use tracing::Level;

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
        let db_path = env::var("RESO_DATABASE_PATH").unwrap_or("reso.db".to_owned());
        let metrics_db_path = env::var("RESO_METRICS_DATABASE_PATH").unwrap_or("reso_metrics.db".to_owned());

        if db_path == metrics_db_path {
            anyhow::bail!("RESO_DATABASE_PATH cannot point to the same path as RESO_METRICS_DATABASE_PATH")
        }

        let dns_server_address = env::var("RESO_DNS_SERVER_ADDRESS").unwrap_or("127.0.0.1:53".to_owned());
        let http_server_address = env::var("RESO_HTTP_SERVER_ADDRESS").unwrap_or("127.0.0.1:80".to_owned());

        // we cannot provide a default for this environment variable.
        let cookie_secret = env::var("RESO_COOKIE_SECRET").map_err(|e| match e {
            VarError::NotPresent => anyhow::anyhow!("Missing RESO_COOKIE_SECRET environment variable"),
            VarError::NotUnicode(_) => {
                anyhow::anyhow!("RESO_COOKIE_SECRET environment variable is in an invalid (non-unicode) format")
            }
        })?;

        let cookie_secret = STANDARD.decode(cookie_secret)?;

        if cookie_secret.len() != 32 {
            anyhow::bail!("Cookie key must exactly have 32 bytes, got {}", cookie_secret.len());
        }

        Ok(Self {
            log_level,
            db_path,
            metrics_db_path: metrics_db_path,
            dns_server_address: SocketAddr::from_str(&dns_server_address)?,
            http_server_address: SocketAddr::from_str(&http_server_address)?,
            cookie_secret,
        })
    }
}
