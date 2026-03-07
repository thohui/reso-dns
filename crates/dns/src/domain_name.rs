use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use idna::AsciiDenyList;

/// Turn raw label bytes into a printable string (RFC 4343 `\DDD` escaping).
///
/// Normal printable ASCII goes through as-is, but weird characters (non-ASCII,
/// control chars, `\`, `.`) gets escaped to `\DDD` decimal form.
/// This is to prevent different byte sequences from looking like the same domain name when turned into a string with lossy utf-8 conversion.
pub fn escape_label(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());

    for &byte in bytes {
        if is_plain_ascii(byte) {
            // normal char, just keep it
            out.push(byte as char);
        } else {
            // not safe to store directly, escape it (e.g. 0xFF → \255)
            write_decimal_escape(&mut out, byte);
        }
    }

    out
}

/// True for printable ASCII that we can keep unescaped.
/// `\` and `.` are excluded since they're meaningful in domain name strings.
fn is_plain_ascii(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7E if byte != b'\\' && byte != b'.')
}

/// Push a `\DDD` escape onto `out` (e.g. `.` => `\046`).
fn write_decimal_escape(out: &mut String, byte: u8) {
    let hundreds = byte / 100;
    let tens = (byte / 10) % 10;
    let ones = byte % 10;

    out.push('\\');
    out.push((b'0' + hundreds) as char);
    out.push((b'0' + tens) as char);
    out.push((b'0' + ones) as char);
}

/// Reverse of `escape_label`, turn an escaped string back into raw bytes.
///
/// `\DDD` becomes a single byte, `\X` becomes `X`, and everything else
/// passes through unchanged. Used when writing labels back to wire format.
pub fn unescape_label(label: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(label.len());
    let raw = label.as_bytes();
    let mut i = 0;

    while i < raw.len() {
        // not a backslash, just a regular char
        if raw[i] != b'\\' {
            out.push(raw[i]);
            i += 1;
            continue;
        }

        // got a backslash, try \DDD (3-digit decimal) first
        if let Some(byte) = try_parse_decimal_escape(raw, i + 1) {
            out.push(byte);
            i += 4; // skip past \ + 3 digits
            continue;
        }

        // not \DDD, so it's \X, just take the next char
        if i + 1 < raw.len() {
            out.push(raw[i + 1]);
            i += 2;
            continue;
        }

        // lone backslash at the very end, nothing to do but keep it
        out.push(b'\\');
        i += 1;
    }

    out
}

/// Try reading 3 ASCII digits at `pos` and turn them into one byte.
/// e.g. "255" at pos => Some(0xFF), "abc" → None.
fn try_parse_decimal_escape(bytes: &[u8], pos: usize) -> Option<u8> {
    if pos + 3 > bytes.len() {
        return None;
    }

    let d1 = ascii_digit_value(bytes[pos])?;
    let d2 = ascii_digit_value(bytes[pos + 1])?;
    let d3 = ascii_digit_value(bytes[pos + 2])?;

    let value = d1 as u16 * 100 + d2 as u16 * 10 + d3 as u16;
    if value > u8::MAX as u16 {
        return None;
    }
    Some(value as u8)
}

/// Convert an ASCII digit char to its numeric value (e.g. b'3' → 3).
/// Returns None if the byte isn't '0'-'9'.
fn ascii_digit_value(byte: u8) -> Option<u8> {
    if byte.is_ascii_digit() { Some(byte - b'0') } else { None }
}

/// Build a display string from raw label bytes (dot-separated, escaped).
fn build_display(labels: &[Box<[u8]>]) -> Arc<str> {
    if labels.is_empty() {
        return Arc::from(".");
    }

    let mut s = String::with_capacity(128); // just a guess to avoid too many reallocations
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            s.push('.');
        }
        s.push_str(&escape_label(label));
    }

    Arc::from(s.as_str())
}

/// A wrapper type for domain names.
///
/// Stores raw label bytes internally, only escaping for display.
/// The labels are stored as lowercase to allow case-insensitive comparisons.
#[derive(Debug, Clone)]
pub struct DomainName {
    /// Raw label bytes (already ASCII-lowercased). Root = empty slice.
    labels: Arc<[Box<[u8]>]>,
    /// Pre computed human-readable form for as_str()/Display/Deref.
    display: Arc<str>,
}

impl Hash for DomainName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.display.hash(state);
    }
}

impl PartialEq for DomainName {
    fn eq(&self, other: &Self) -> bool {
        self.display == other.display
    }
}

impl Eq for DomainName {}

impl DomainName {
    /// Primary constructor from raw label byte vectors.
    ///
    /// Validates label lengths, lowercases ASCII, and builds the display string.
    pub fn from_labels(raw_labels: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        if raw_labels.is_empty() {
            return Ok(Self::root());
        }

        let mut wire_len: usize = 1; // trailing root label (0 byte)
        let mut labels = Vec::with_capacity(raw_labels.len());

        for mut label in raw_labels {
            if label.is_empty() {
                anyhow::bail!("empty domain label");
            }
            if label.len() > 63 {
                anyhow::bail!("domain label too long: {} bytes", label.len());
            }

            wire_len += 1 + label.len(); // length byte + label data

            label.make_ascii_lowercase();
            labels.push(label.into_boxed_slice());
        }

        if wire_len > 255 {
            anyhow::bail!("domain name too long: {} wire bytes", wire_len);
        }

        let display = build_display(&labels);
        let labels: Arc<[Box<[u8]>]> = Arc::from(labels);

        Ok(Self { labels, display })
    }

    /// Create a new Qname from an ASCII string.
    /// The domain name is validated according to RFC 1035.
    ///
    /// NOTE: This function does not support Unicode domain names and should only be called with ASCII input.
    pub fn from_ascii(s: impl AsRef<str>) -> anyhow::Result<Self> {
        let s = s.as_ref();

        // Handle root
        if s == "." || s.is_empty() {
            return Ok(Self::root());
        }

        // Remove trailing dot if present.
        let s = s.strip_suffix('.').unwrap_or(s);

        let raw_labels: Vec<Vec<u8>> = s.split('.').map(unescape_label).collect();

        // Validate no empty labels
        for (i, label) in raw_labels.iter().enumerate() {
            if label.is_empty() {
                anyhow::bail!("empty domain label in: {}", s.split('.').nth(i).unwrap_or(""));
            }
        }

        Self::from_labels(raw_labels)
    }

    /// Create a new Qname from a user input string.
    /// This function supports Unicode domain names and performs IDNA conversion.
    pub fn from_user(s: impl AsRef<str>) -> anyhow::Result<Self> {
        let input = s.as_ref().trim();

        if input == "." {
            return Ok(Self::root());
        }

        let name = if input.ends_with('.') {
            input.strip_suffix('.').unwrap_or(input)
        } else {
            input
        };

        // IDNA to ASCII
        let ascii = idna::domain_to_ascii_cow(name.as_bytes(), AsciiDenyList::URL)
            .map_err(|_| anyhow::anyhow!("invalid IDNA domain: {}", input))?;

        Self::from_ascii(&ascii)
    }

    /// Create a root `DomainName`
    pub fn root() -> Self {
        Self {
            labels: Arc::from([]),
            display: Arc::from("."),
        }
    }

    /// Returns true if this is the root domain name.
    pub fn is_root(&self) -> bool {
        self.labels.is_empty()
    }

    /// Get the string representation of the DomainName.
    pub fn as_str(&self) -> &str {
        &self.display
    }

    /// Exact wire format byte count for this domain name.
    pub fn wire_len(&self) -> usize {
        if self.labels.is_empty() {
            return 1; // just the root label (0 byte)
        }
        // each label: 1 length byte + data bytes, plus 1 trailing root byte
        self.labels.iter().map(|l| 1 + l.len()).sum::<usize>() + 1
    }

    /// Get an iterator of raw label bytes.
    pub fn label_iter(&self) -> impl Iterator<Item = &[u8]> {
        self.labels.iter().map(|l| l.as_ref())
    }
}

impl Deref for DomainName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl Display for DomainName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
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

    #[test]
    fn test_from_labels() {
        let labels = vec![b"example".to_vec(), b"com".to_vec()];
        let dn = DomainName::from_labels(labels).unwrap();
        assert_eq!(dn.as_str(), "example.com");
    }

    #[test]
    fn test_from_labels_lowercases() {
        let labels = vec![b"EXAMPLE".to_vec(), b"COM".to_vec()];
        let dn = DomainName::from_labels(labels).unwrap();
        assert_eq!(dn.as_str(), "example.com");
    }

    #[test]
    fn test_from_labels_empty_is_root() {
        let dn = DomainName::from_labels(vec![]).unwrap();
        assert_eq!(dn.as_str(), ".");
        assert!(dn.is_root());
    }

    #[test]
    fn test_from_labels_rejects_empty_label() {
        assert!(DomainName::from_labels(vec![vec![]]).is_err());
    }

    #[test]
    fn test_from_labels_rejects_long_label() {
        assert!(DomainName::from_labels(vec![vec![0x41; 64]]).is_err());
    }

    #[test]
    fn test_wire_len() {
        let root = DomainName::root();
        assert_eq!(root.wire_len(), 1);

        let dn = DomainName::from_ascii("example.com").unwrap();
        // 1 + 7 + 1 + 3 + 1 = 13
        assert_eq!(dn.wire_len(), 13);
    }

    #[test]
    fn test_is_root() {
        assert!(DomainName::root().is_root());
        assert!(!DomainName::from_ascii("example.com").unwrap().is_root());
    }

    #[test]
    fn test_label_iter_returns_raw_bytes() {
        let labels = vec![vec![0x80, 0xFF], b"com".to_vec()];
        let dn = DomainName::from_labels(labels).unwrap();
        let collected: Vec<&[u8]> = dn.label_iter().collect();
        assert_eq!(collected, vec![&[0x80, 0xFF][..], b"com"]);
    }

    #[test]
    fn test_hash_eq_based_on_labels() {
        use std::collections::HashSet;
        let dn1 = DomainName::from_ascii("example.com").unwrap();
        let dn2 = DomainName::from_ascii("EXAMPLE.COM").unwrap();
        assert_eq!(dn1, dn2);

        let mut set = HashSet::new();
        set.insert(dn1.clone());
        assert!(set.contains(&dn2));
    }
}
