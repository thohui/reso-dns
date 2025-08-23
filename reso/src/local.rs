#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Local {
    /// Whether the response was cached or not.
    pub cache_hit: bool,
}
