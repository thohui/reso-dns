use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

use idna::AsciiDenyList;

/// A wrapper type for domain names.
/// The input is stored as lowercase to allow case-insensitive comparisons.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DomainName(Arc<str>);

impl DomainName {
    /// Create a new Qname from an ASCII string.
    /// The domain name is validated according to RFC 1035.
    ///
    /// NOTE: This function does not support Unicode domain names and should only be called with ASCII input.
    pub fn from_ascii(s: impl AsRef<str>) -> anyhow::Result<Self> {
        let mut str: String = s.as_ref().into();

        // Handle root
        if &str == "." || str.is_empty() {
            return Ok(Self(Arc::from(".")));
        }

        // Remove trailing dot if present.
        if str.ends_with('.') {
            str.pop();
        }

        let bytes = str.as_bytes();

        // Validate the domain name according to RFC 1035.
        if bytes.len() > 255 {
            anyhow::bail!("domain name too long (bytes): {}", str);
        }

        for label in str.split('.') {
            // No empty labels allowed (except for root, which is handled above).
            if label.is_empty() {
                anyhow::bail!("empty domain label in: {}", str);
            }

            // Every label must be between 1 and 63 characters long (RFC 1035).
            if label.len() > 63 {
                anyhow::bail!("domain label too long: {}", label);
            }
        }

        str.make_ascii_lowercase();

        Ok(Self(Arc::from(str.trim())))
    }

    /// Create a new Qname from a user input string.
    /// This function supports Unicode domain names and performs IDNA conversion.
    pub fn from_user(s: impl AsRef<str>) -> anyhow::Result<Self> {
        let input = s.as_ref().trim();

        if input == "." {
            return Ok(Self(Arc::from(".")));
        }

        let name = if input.ends_with('.') {
            input.strip_suffix('.').unwrap()
        } else {
            input
        };

        // IDNA to ASCII
        let ascii = idna::domain_to_ascii_cow(name.as_bytes(), AsciiDenyList::URL)
            .map_err(|_| anyhow::anyhow!("invalid IDNA domain: {}", input))?;

        Self::from_ascii(&ascii)
    }

    /// Get the string representation of the DomainName.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for DomainName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for DomainName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_name_creation() {
        let dn = DomainName::from_ascii("Example.com.").unwrap();
        assert_eq!(dn.as_str(), "example.com");

        let dn2 = DomainName::from_ascii("sub.domain.example.com").unwrap();
        assert_eq!(dn2.as_str(), "sub.domain.example.com");

        assert!(DomainName::from_ascii("a".repeat(256)).is_err());
        assert!(DomainName::from_ascii("label..example.com").is_err());
        assert!(DomainName::from_ascii("a".repeat(64) + ".com").is_err());
    }
}
