use aes_gcm::{
    aead::{OsRng, rand_core::RngCore},
    aes::Aes256,
};
use axum_extra::extract::cookie;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    env::{self, VarError},
    error::Error,
    net::SocketAddr,
    str::FromStr,
};
use tracing::Level;

/// Errors that can occur when loading the config.
#[derive(Debug)]
pub enum ConfigError {
    /// Config file not found.
    NotFound,
    /// Failed to decode config file.
    Decode(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("config file not found"),
            Self::Decode(e) => f.write_str(e),
        }
    }
}

impl Error for ConfigError {}

/// Cookie encryption key
fn generate_cookie_key() -> [u8; 32] {
    let mut key = [0u8; 32]; // AES-256 key
    OsRng.fill_bytes(&mut key);
    key
}

mod base64_32 {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(key: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = STANDARD.encode(key);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = STANDARD.decode(s.trim()).map_err(serde::de::Error::custom)?;

        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "cookie_key must decode to 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(out)
    }
}

pub struct EnvConfig {
    pub log_level: Level,
    pub db_path: String,
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
        let dns_server_address = env::var("RESO_DNS_SERVER_ADDRESS").unwrap_or("0.0.0.0:53".to_owned());
        let http_server_address = env::var("RESO_HTTP_SERVER_ADDRESS").unwrap_or("0.0.0.0:80".to_owned());

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
            dns_server_address: SocketAddr::from_str(&dns_server_address)?,
            http_server_address: SocketAddr::from_str(&http_server_address)?,
            cookie_secret,
        })
    }
}
