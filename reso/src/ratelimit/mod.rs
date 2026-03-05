use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use moka::future::Cache;
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
        let mut entry = match self.windows.get(&ip).await {
            Some(entry) => entry,
            None => {
                let entry = RateWindow {
                    start: now,
                    query_count: 1,
                };
                self.windows.insert(ip, entry.clone()).await;
                return true;
            }
        };

        if now.duration_since(entry.start) >= self.config.window_duration {
            self.windows.invalidate(&ip).await;
            true
        } else {
            if entry.query_count < self.config.max_queries_per_window {
                entry.query_count += 1;
                self.windows.insert(ip, entry).await;
                true
            } else {
                false
            }
        }
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
