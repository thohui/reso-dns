use std::time::{Duration, Instant};

use crate::{database::models::domain_rule::DomainRule, uuid::EntityId};

/// Local state for a DNS request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    /// Whether the response was served from cache.
    pub cache_hit: bool,
    /// Whether the request was blocked.
    pub blocked: bool,
    /// When the request was started
    pub time_started: Instant,
    /// Whether the request was rate limited.
    pub rate_limited: bool,
    pub rule_id: Option<EntityId<DomainRule>>,
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
            blocked: Default::default(),
            time_started: Instant::now(),
            rate_limited: Default::default(),
            rule_id: None,
        }
    }
}
