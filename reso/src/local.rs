/// Local state for a DNS request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Local {
    /// Whether the response was served from cache.
    pub cache_hit: bool,

    /// Whether the metrics have already been recorded.
    pub metrics_recorded: bool,

    /// Whether the request was blocked.
    pub blocked: bool,
}
