use serde::Deserialize;
use std::net::SocketAddr;
use tracing::{Level, level_filters::LevelFilter};

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "debug")]
    Debug,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_ip")]
    pub ip: String,
    #[serde(default = "default_server_port")]
    pub port: u64,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolverConfig {
    Forwarder {
        #[serde(default)]
        upstreams: Vec<SocketAddr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

fn default_db_path() -> String {
    "database.db".into()
}

fn default_server_ip() -> String {
    "0.0.0.0".into()
}

fn default_server_port() -> u64 {
    53
}

fn default_log_level() -> LogLevel {
    LogLevel::Info
}
