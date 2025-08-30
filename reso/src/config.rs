use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::{Level, level_filters::LevelFilter};

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum LogLevel {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "debug")]
    Debug,
    #[default]
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

impl From<LogLevel> for Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_ip")]
    pub ip: String,
    #[serde(default = "default_server_port")]
    pub port: u64,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: default_server_ip(),
            port: default_server_port(),
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolverConfig {
    Forwarder {
        #[serde(default)]
        upstreams: Vec<SocketAddr>,
    },
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self::Forwarder { upstreams: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub resolver: ResolverConfig,
}

pub fn decode_from_path(path: &str) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn create_default_config() -> anyhow::Result<Config> {
    let cfg = Config {
        server: ServerConfig::default(),
        database: DatabaseConfig::default(),
        resolver: ResolverConfig::default(),
    };

    let toml_str = toml::to_string_pretty(&cfg)?;

    std::fs::write(DEFAULT_CONFIG_PATH, toml_str)?;

    Ok(cfg)
}

fn default_db_path() -> String {
    "reso.db".into()
}

fn default_server_ip() -> String {
    "0.0.0.0".into()
}

fn default_server_port() -> u64 {
    53
}

fn default_log_level() -> LogLevel {
    LogLevel::default()
}
