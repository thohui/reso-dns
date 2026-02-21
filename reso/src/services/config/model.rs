use std::net::SocketAddr;

use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Config
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub dns: DnsConfig,
}

#[derive(Serialize, Deserialize)]
pub struct DnsConfig {
    /// Timeout for dns queries in milliseconds.
    pub timeout: u64,
    // The currently active resolver.
    pub active: ActiveResolver,
    /// Forwarder config.
    pub forwarder: ForwarderConfig,
}

#[derive(Serialize, Deserialize)]
pub enum ActiveResolver {
    #[serde(rename = "forwarder")]
    Forwarder,
}
#[derive(Serialize, Deserialize)]
pub struct ForwarderConfig {
    pub upstreams: Vec<SocketAddr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dns: DnsConfig {
                timeout: Duration::seconds(3).num_milliseconds() as u64,
                active: ActiveResolver::Forwarder,
                forwarder: ForwarderConfig { upstreams: vec![] },
            },
        }
    }
}
