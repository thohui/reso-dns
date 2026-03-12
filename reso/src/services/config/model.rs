use std::time::Duration;
use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::ratelimit;

/// Config
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub dns: DnsConfig,
    pub logs: LogsConfig,
}

#[derive(Serialize, Deserialize)]
pub struct LogsConfig {
    /// Whether log truncation is enabled.
    pub enabled: bool,
    /// How long to keep activity logs in seconds.
    pub retention_secs: u64,
    /// How often to run the truncation job in seconds.
    pub truncate_interval_secs: u64,
}

#[derive(Serialize, Deserialize)]
pub struct DnsConfig {
    /// Timeout for dns queries in milliseconds.
    pub timeout: u64,
    /// The currently active resolver.
    pub active: ActiveResolver,
    /// Forwarder config.
    pub forwarder: ForwarderConfig,
    /// Rate limit config.
    pub rate_limit: RateLimitConfigModel,
}

#[derive(Serialize, Deserialize)]
pub enum ActiveResolver {
    #[serde(rename = "forwarder")]
    Forwarder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfigModel {
    /// Enabled
    pub enabled: bool,
    /// Duration of each rate limit window in seconds.
    pub window_duration: usize,
    /// Maximum number of queries allowed per window.
    pub max_queries_per_window: usize,
}

impl From<ratelimit::RateLimitConfig> for RateLimitConfigModel {
    fn from(config: ratelimit::RateLimitConfig) -> Self {
        Self {
            enabled: false,
            window_duration: config.window_duration.as_secs() as usize,
            max_queries_per_window: config.max_queries_per_window,
        }
    }
}

/// Runtime endpoint type (hostname or IP + port).
#[derive(Debug, Clone)]
pub struct HostPort {
    pub host: String,
    pub port: u16,
}

impl HostPort {
    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(SocketAddr::from_str(&format!("{}:{}", self.host, self.port))?)
    }
}

#[derive(Debug, Clone)]
pub enum Upstream {
    /// UDP and TCP
    Plain { endpoint: HostPort },
    /// DNS over TLS
    Tls { endpoint: HostPort },
    /// DNS over Https
    Doh { url: Url },
}

/// String representation of an `Upstream`.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UpstreamSpec(pub String);

impl UpstreamSpec {
    pub fn parse(&self) -> Result<Upstream> {
        let s = self.0.trim();

        if s.starts_with("https://") || s.starts_with("http://") {
            let url = Url::parse(s).context("invalid DoH URL")?;
            return Ok(Upstream::Doh { url });
        }

        let (scheme, rest) = match s.split_once("://") {
            Some((sch, rest)) => (sch, rest),
            None => ("plain", s),
        };

        let (host, port_opt) = split_host_port(rest).context("invalid host[:port]")?;

        let (default_port, make): (u16, fn(HostPort) -> Upstream) = match scheme {
            "plain" => (53, |hp| Upstream::Plain { endpoint: hp }),
            "udp" => (53, |hp| Upstream::Plain { endpoint: hp }),
            "tcp" => (53, |hp| Upstream::Plain { endpoint: hp }),
            "tls" => (853, |hp| Upstream::Tls { endpoint: hp }),
            other => bail!("unsupported scheme: {other}"),
        };

        let endpoint = HostPort {
            host,
            port: port_opt.unwrap_or(default_port),
        };

        Ok(make(endpoint))
    }
}

fn split_host_port(s: &str) -> Result<(String, Option<u16>)> {
    let s = s.trim();
    if s.is_empty() {
        bail!("empty upstream");
    }

    if let Some((host, port)) = s.rsplit_once(':')
        && !host.contains(':')
        && !host.is_empty()
    {
        let port: u16 = port.parse().with_context(|| format!("invalid port: {port:?}"))?;
        return Ok((host.to_string(), Some(port)));
    }

    Ok((s.to_string(), None))
}

#[derive(Serialize, Deserialize)]
pub struct ForwarderConfig {
    pub upstreams: Vec<UpstreamSpec>,
}

impl ForwarderConfig {
    pub fn upstreams(&self) -> anyhow::Result<Vec<Upstream>> {
        self.upstreams
            .iter()
            .enumerate()
            .map(|(i, spec)| spec.parse().with_context(|| format!("forwarder.upstreams[{i}]")))
            .collect()
    }
}

impl Config {
    pub fn from_kv(map: &HashMap<String, String>) -> Self {
        let defaults = Self::default();

        let timeout = map
            .get("dns.timeout")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(defaults.dns.timeout);

        let active = map
            .get("dns.active")
            .and_then(|v| serde_json::from_value::<ActiveResolver>(serde_json::Value::String(v.clone())).ok())
            .unwrap_or(defaults.dns.active);

        let upstreams = map
            .get("dns.forwarder.upstreams")
            .and_then(|v| serde_json::from_str::<Vec<String>>(v).ok())
            .map(|specs| specs.into_iter().map(UpstreamSpec).collect())
            .unwrap_or(defaults.dns.forwarder.upstreams);

        let rate_limit_enabled = map
            .get("dns.rate_limit.enabled")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(defaults.dns.rate_limit.enabled);

        let window_duration = map
            .get("dns.rate_limit.window_duration")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(defaults.dns.rate_limit.window_duration);

        let max_queries_per_window = map
            .get("dns.rate_limit.max_queries_per_window")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(defaults.dns.rate_limit.max_queries_per_window);

        let logs_enabled = map
            .get("logs.enabled")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(defaults.logs.enabled);

        let retention_secs = map
            .get("logs.retention_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(defaults.logs.retention_secs);

        let truncate_interval_secs = map
            .get("logs.truncate_interval_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(defaults.logs.truncate_interval_secs);

        Self {
            dns: DnsConfig {
                timeout,
                active,
                forwarder: ForwarderConfig { upstreams },
                rate_limit: RateLimitConfigModel {
                    enabled: rate_limit_enabled,
                    window_duration,
                    max_queries_per_window,
                },
            },
            logs: LogsConfig {
                enabled: logs_enabled,
                retention_secs,
                truncate_interval_secs,
            },
        }
    }

    pub fn to_kv(&self) -> Vec<(String, String)> {
        let active_str = match &self.dns.active {
            ActiveResolver::Forwarder => "forwarder",
        };

        let upstreams_json =
            serde_json::to_string(&self.dns.forwarder.upstreams.iter().map(|u| &u.0).collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string());

        vec![
            ("dns.timeout".to_string(), self.dns.timeout.to_string()),
            ("dns.active".to_string(), active_str.to_string()),
            ("dns.forwarder.upstreams".to_string(), upstreams_json),
            (
                "dns.rate_limit.enabled".to_string(),
                self.dns.rate_limit.enabled.to_string(),
            ),
            (
                "dns.rate_limit.window_duration".to_string(),
                self.dns.rate_limit.window_duration.to_string(),
            ),
            (
                "dns.rate_limit.max_queries_per_window".to_string(),
                self.dns.rate_limit.max_queries_per_window.to_string(),
            ),
            ("logs.enabled".to_string(), self.logs.enabled.to_string()),
            ("logs.retention_secs".to_string(), self.logs.retention_secs.to_string()),
            (
                "logs.truncate_interval_secs".to_string(),
                self.logs.truncate_interval_secs.to_string(),
            ),
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dns: DnsConfig {
                timeout: Duration::from_secs(3).as_millis() as u64,
                active: ActiveResolver::Forwarder,
                forwarder: ForwarderConfig { upstreams: vec![] },
                rate_limit: RateLimitConfigModel {
                    enabled: false,
                    window_duration: Duration::from_secs(10).as_secs() as usize,
                    max_queries_per_window: 100,
                },
            },
            logs: LogsConfig {
                enabled: false,
                retention_secs: 7 * 24 * 3600,
                truncate_interval_secs: 3600,
            },
        }
    }
}
