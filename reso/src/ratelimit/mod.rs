use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

pub struct RateLimiter {
    windows: DashMap<IpAddr, RateWindow>,
    config: RateLimitConfig,
}

struct RateWindow {
    start: Instant,
    query_count: usize,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            windows: DashMap::new(),
            config,
        }
    }

    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        let mut entry = self.windows.entry(ip).or_insert_with(|| RateWindow {
            start: now,
            query_count: 0,
        });

        if now.duration_since(entry.start) > self.config.window_duration {
            entry.start = now;
            entry.query_count = 1;
            true
        } else {
            if entry.query_count < self.config.max_queries_per_window {
                entry.query_count += 1;
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
