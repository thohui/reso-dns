use std::time::{Duration, Instant};

/// Local state for a DNS request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    /// Whether the response was served from cache.
    pub cache_hit: bool,

    /// Whether the metrics have already been recorded.
    pub metrics_recorded: bool,

    /// Whether the request was blocked.
    pub blocked: bool,

    /// When the request was started
    pub time_started: Instant,
}

impl Local {
    pub fn time_elapsed(&self) -> Duration {
        let now = Instant::now();
        now - self.time_started
    }
}

impl Default for Local {
    fn default() -> Self {
        Self {
            cache_hit: Default::default(),
            metrics_recorded: Default::default(),
            blocked: Default::default(),
            time_started: Instant::now(),
        }
    }
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod local_tests;