use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_ip")]
    pub ip: String,
    #[serde(default = "default_server_port")]
    pub port: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
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
