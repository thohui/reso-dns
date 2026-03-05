use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use moka::{future::Cache, ops::compute::Op};
use serde::{Deserialize, Serialize};

pub struct RateLimiter {
    windows: Cache<IpAddr, RateWindow>,
    config: RateLimitConfig,
}

#[derive(Clone, Debug)]
struct RateWindow {
    start: Instant,
    query_count: usize,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            windows: Cache::builder().time_to_live(Duration::from_mins(1)).build(),
            config,
        }
    }

    pub async fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window_duration = self.config.window_duration;
        let max_queries = self.config.max_queries_per_window;

        let result = self
            .windows
            .entry(ip)
            .and_compute_with(|maybe_entry| async move {
                match maybe_entry {
                    Some(entry) => {
                        let window = entry.into_value();
                        if now.duration_since(window.start) >= window_duration {
                            Op::Put(RateWindow {
                                start: now,
                                query_count: 1,
                            })
                        } else if window.query_count < max_queries {
                            Op::Put(RateWindow {
                                query_count: window.query_count + 1,
                                ..window
                            })
                        } else {
                            Op::Nop
                        }
                    }
                    None => Op::Put(RateWindow {
                        start: now,
                        query_count: 1,
                    }),
                }
            })
            .await;

        !matches!(result, moka::ops::compute::CompResult::Unchanged(_))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Duration of each rate limit window in seconds.
    pub window_duration: Duration,
    pub max_queries_per_window: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_secs(30),
            max_queries_per_window: 100,
        }
    }
}

impl RateLimitConfig {
    pub fn new(window_duration: Duration, max_queries_per_window: usize) -> Self {
        Self {
            window_duration,
            max_queries_per_window,
        }
    }
}
