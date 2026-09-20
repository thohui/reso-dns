use crate::error::{DnsReadError, ReadResult};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use idna::AsciiDenyList;

// \ and . are excluded since they're meaningful in domain name strings
fn is_plain_ascii(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7E if byte != b'\\' && byte != b'.')
}

fn write_decimal_escape(out: &mut String, byte: u8) {
    let hundreds = byte / 100;
    let tens = (byte / 10) % 10;
    let ones = byte % 10;

    out.push('\\');
    out.push((b'0' + hundreds) as char);
    out.push((b'0' + tens) as char);
    out.push((b'0' + ones) as char);
}

pub fn unescape_label(label: &str) -> Vec<u8> {
    // \DDD becomes a byte value and \X becomes X
    let mut out = Vec::with_capacity(label.len());
    let raw = label.as_bytes();
    let mut i = 0;

    while i < raw.len() {
        if raw[i] != b'\\' {
            out.push(raw[i]);
            i += 1;
            continue;
        }

        if let Some(byte) = try_parse_decimal_escape(raw, i + 1) {
            out.push(byte);
            i += 4; // skip \ + 3 digits
            continue;
        }

        if i + 1 < raw.len() {
            out.push(raw[i + 1]);
            i += 2;
            continue;
        }

        // lone backslash at end
        out.push(b'\\');
        i += 1;
    }

    out
}

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

fn ascii_digit_value(byte: u8) -> Option<u8> {
    if byte.is_ascii_digit() { Some(byte - b'0') } else { None }
}

struct LabelIter<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for LabelIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let len = *self.data.first()? as usize;
        if len == 0 {
            return None;
        }
        self.data = &self.data[1..];
        let label = &self.data[..len];
        self.data = &self.data[len..];
        Some(label)
    }
}

/// Labels are stored in DNS wire format, lowercased for case-insensitive comparison
#[derive(Clone)]
pub struct DomainName {
    inner: Arc<Inner>,
}

#[derive(Clone)]
struct Inner {
    labels: Box<[u8]>,
    display: OnceLock<Box<str>>,
}

impl Hash for DomainName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.labels.hash(state);
    }
}

impl PartialEq for DomainName {
    fn eq(&self, other: &Self) -> bool {
        self.inner.labels.eq(&other.inner.labels)
    }
}

impl Eq for DomainName {}

impl std::fmt::Debug for DomainName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DomainName").field(&self.as_str()).finish()
    }
}

fn render(labels: &[u8]) -> Box<str> {
    if labels.len() <= 1 {
        return Box::from(".");
    }

    let mut out = String::with_capacity(labels.len());
    for (i, label) in (LabelIter { data: labels }).enumerate() {
        if i > 0 {
            out.push('.');
        }
        for &byte in label {
            if is_plain_ascii(byte) {
                out.push(byte as char);
            } else {
                write_decimal_escape(&mut out, byte);
            }
        }
    }
    out.into_boxed_str()
}

impl DomainName {
    pub fn from_labels<L: AsRef<[u8]>>(raw_labels: &[L]) -> ReadResult<Self> {
        let mut wire_len: usize = 1; // 1 for root terminator

        for item in raw_labels {
            let label = item.as_ref();

            if label.is_empty() {
                return Err(DnsReadError::EmptyLabel);
            }
            if label.len() > 63 {
                return Err(DnsReadError::LabelTooLong { len: label.len() });
            }

            wire_len += 1 + label.len();
            if wire_len > 255 {
                return Err(DnsReadError::NameTooLong { len: wire_len });
            }
        }

        if raw_labels.is_empty() {
            return Ok(Self::root());
        }

        let mut wire: Vec<u8> = Vec::with_capacity(wire_len);

        for item in raw_labels {
            let label = item.as_ref();
            wire.push(label.len() as u8);
            let label_start = wire.len();
            wire.extend_from_slice(label);
            wire[label_start..].make_ascii_lowercase();
        }

        wire.push(0); // root label terminator

        Ok(Self {
            inner: Arc::new(Inner {
                labels: wire.into_boxed_slice(),
                display: OnceLock::new(),
            }),
        })
    }

    pub fn from_ascii(s: impl AsRef<str>) -> ReadResult<Self> {
        let s = s.as_ref();

        if s == "." || s.is_empty() {
            return Ok(Self::root());
        }

        let s = s.strip_suffix('.').unwrap_or(s);

        let raw_labels: Vec<Vec<u8>> = s.split('.').map(unescape_label).collect();
        Self::from_labels(&raw_labels)
    }

    pub fn from_user(s: impl AsRef<str>) -> ReadResult<Self> {
        let input = s.as_ref().trim();

        if input == "." {
            return Ok(Self::root());
        }

        let name = if input.ends_with('.') {
            input.strip_suffix('.').unwrap_or(input)
        } else {
            input
        };

        let ascii =
            idna::domain_to_ascii_cow(name.as_bytes(), AsciiDenyList::URL).map_err(|e| DnsReadError::InvalidIdna {
                input: input.to_string(),
                cause: e,
            })?;

        Self::from_ascii(&ascii)
    }

    pub fn root() -> Self {
        Self {
            inner: Arc::new(Inner {
                labels: Box::from([0u8].as_slice()),
                display: OnceLock::new(),
            }),
        }
    }

    pub fn is_root(&self) -> bool {
        self.inner.labels.len() == 1
    }

    pub fn as_str(&self) -> &str {
        self.inner.display.get_or_init(|| render(&self.inner.labels))
    }

    pub fn wire_len(&self) -> usize {
        self.inner.labels.len()
    }

    pub fn label_iter(&self) -> impl Iterator<Item = &[u8]> {
        LabelIter {
            data: &self.inner.labels,
        }
    }
}

impl Deref for DomainName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Display for DomainName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
        let dn = DomainName::from_labels(&labels).unwrap();
        assert_eq!(dn.as_str(), "example.com");
    }

    #[test]
    fn test_from_labels_lowercases() {
        let labels = vec![b"EXAMPLE".to_vec(), b"COM".to_vec()];
        let dn = DomainName::from_labels(&labels).unwrap();
        assert_eq!(dn.as_str(), "example.com");
    }

    #[test]
    fn test_from_labels_empty_is_root() {
        let dn = DomainName::from_labels(&Vec::<Vec<u8>>::new()).unwrap();
        assert_eq!(dn.as_str(), ".");
        assert!(dn.is_root());
    }

    #[test]
    fn test_from_labels_rejects_empty_label() {
        assert!(DomainName::from_labels(&[vec![]]).is_err());
    }

    #[test]
    fn test_from_labels_rejects_long_label() {
        assert!(DomainName::from_labels(&[vec![0x41; 64]]).is_err());
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
        let dn = DomainName::from_labels(&labels).unwrap();
        let collected: Vec<&[u8]> = dn.label_iter().collect();
        assert_eq!(collected, vec![&[0x80, 0xFF][..], b"com"]);
    }

    #[test]
    fn test_hash_eq_based_on_labels() {
        use std::collections::HashSet;
        let dn1 = DomainName::from_ascii("example.com").unwrap();
        let dn2 = DomainName::from_ascii("EXAMPLE.COM").unwrap();
        assert_eq!(dn1, dn2);

        #[allow(clippy::mutable_key_type)]
        let mut set = HashSet::new();
        set.insert(dn1.clone());
        assert!(set.contains(&dn2));
    }
}
