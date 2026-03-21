use sha2::{Digest, Sha256};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ListFormat {
    /// 0.0.0.0 domain.com
    Hosts,
    /// domain.com (one per line)
    Plain,
    // TODO: add support for more formats like adblock plus.
}

/// A simple parser for blocklist content in various formats.
pub struct ListParser<'a> {
    content: &'a str,
}

impl<'a> ListParser<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }

    pub fn parse(&self) -> Vec<&'a str> {
        let format = detect_format(self.content).unwrap_or(ListFormat::Plain);
        self.content
            .lines()
            .filter_map(|line| match format {
                ListFormat::Hosts => parse_hosts_line(line),
                ListFormat::Plain => parse_plain_line(line),
            })
            .collect()
    }
}

pub fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn detect_format(content: &str) -> Option<ListFormat> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_ascii_whitespace();
        if matches!(parts.next(), Some("0.0.0.0" | "127.0.0.1")) {
            return Some(ListFormat::Hosts);
        }
        return Some(ListFormat::Plain);
    }
    None
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn validate(s: &str) -> Option<&str> {
    if s.is_empty() || s.len() > 253 {
        return None;
    }

    // basic validation, the full validation is up to the consumer.
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '*'))
    {
        return None;
    }

    Some(s)
}

const LOCAL_DOMAINS: &[&str] = &[
    "localhost",
    "local",
    "broadcasthost",
    "localhost.localdomain",
    "ip6-localhost",
    "ip6-loopback",
    "ip6-localnet",
    "ip6-allnodes",
    "ip6-allrouters",
];

fn parse_hosts_line(line: &str) -> Option<&str> {
    let line = strip_comment(line).trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split_ascii_whitespace();
    parts.next()?; // skip ip addr
    let domain = parts.next()?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    validate(domain)
}

fn parse_plain_line(line: &str) -> Option<&str> {
    let line = strip_comment(line).trim();
    let mut parts = line.split_ascii_whitespace();
    let domain = parts.next()?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    validate(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/hosts.txt"));
    const PLAIN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plain.txt"));

    #[test]
    fn detects_hosts_format() {
        assert!(matches!(detect_format(HOSTS), Some(ListFormat::Hosts)));
    }

    #[test]
    fn detects_plain_format() {
        assert!(matches!(detect_format(PLAIN), Some(ListFormat::Plain)));
    }

    #[test]
    fn detects_none_for_empty() {
        assert!(matches!(detect_format(""), None));
        assert!(matches!(detect_format("# just a comment\n"), None));
    }

    #[test]
    fn parses_hosts() {
        let domains = ListParser::new(HOSTS).parse();
        assert!(domains.contains(&"ads.example.com"));
        assert!(domains.contains(&"tracker.example.com"));
        assert!(domains.contains(&"telemetry.example.com"));
        assert!(domains.contains(&"metrics.example.com"));
        assert!(domains.contains(&"spacing.example.com"));
        assert!(domains.contains(&"another.example.com"));
    }

    #[test]
    fn hosts_filters_local_domains() {
        let domains = ListParser::new(HOSTS).parse();
        assert!(!domains.contains(&"localhost"));
        assert!(!domains.contains(&"localhost.localdomain"));
        assert!(!domains.contains(&"local"));
        assert!(!domains.contains(&"broadcasthost"));
        assert!(!domains.contains(&"ip6-localhost"));
        assert!(!domains.contains(&"ip6-loopback"));
    }

    #[test]
    fn parses_plain() {
        let domains = ListParser::new(PLAIN).parse();
        assert!(domains.contains(&"ads.example.com"));
        assert!(domains.contains(&"tracker.example.com"));
        assert!(domains.contains(&"telemetry.example.com"));
        assert!(domains.contains(&"metrics.example.com"));
        assert!(domains.contains(&"*.ads.example.com"));
        assert!(domains.contains(&"another.example.com"));
    }
}
