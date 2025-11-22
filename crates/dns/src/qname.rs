use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

/// A wrapper type for domain names.
/// The input is stored as lowercase to allow case-insensitive comparisons.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Qname(Arc<str>);

impl Qname {
    /// Create a new Qname from a string.
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        let lowercased = s.as_ref().to_ascii_lowercase();
        Self(Arc::from(lowercased))
    }

    /// Get the string representation of the Qname.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Qname {
    fn from(s: &str) -> Self {
        Qname::new(s)
    }
}

impl Deref for Qname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Qname {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
