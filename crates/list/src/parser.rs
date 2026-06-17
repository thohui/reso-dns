#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ListFormat {
    /// 0.0.0.0 domain.com
    Hosts,
    /// domain.com (one per line)
    Plain,
    // TODO: add support for more formats like adblock plus.
}

/// Parser for blocklist content in various formats.
pub struct ListParser {
    pub format: Option<ListFormat>,
    leftover: String,
}

impl Default for ListParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ListParser {
    pub fn new() -> Self {
        Self {
            format: None,
            leftover: String::new(),
        }
    }

    /// Process a text chunk, calling `callback` for each parsed domain.
    pub fn push<F: FnMut(&str)>(&mut self, chunk: &str, mut callback: F) {
        self.leftover.push_str(chunk);

        let mut start = 0;
        while let Some(rel_pos) = self.leftover[start..].find('\n') {
            let end = start + rel_pos;
            let line = self.leftover[start..end].trim_end_matches('\r');

            // detect format from first non comment or non empty line
            if self.format.is_none() {
                self.format = detect_line_format(line);
            }
            if let Some(fmt) = self.format
                && let Some(domain) = parse_line(line, fmt)
            {
                callback(domain);
            }

            start = end + 1;
        }

        // keep the incomplete line for the next chunk
        self.leftover.drain(..start);
    }

    /// Process any remaining incomplete line after the last chunk.
    pub fn flush<F: FnMut(&str)>(mut self, mut callback: F) {
        if !self.leftover.is_empty() {
            // file with no trailing newline
            let leftover = std::mem::take(&mut self.leftover);
            let line = leftover.trim_end_matches('\r');
            if self.format.is_none() {
                self.format = detect_line_format(line);
            }
            if let Some(fmt) = self.format
                && let Some(domain) = parse_line(line, fmt)
            {
                callback(domain);
            }
        }
    }
}

/// Detect the format from a single line.
/// Returns `None` for blank/comment lines or unrecognized formats.
fn detect_line_format(line: &str) -> Option<ListFormat> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut parts = line.split_ascii_whitespace();
    let first = parts.next()?;
    if matches!(first, "0.0.0.0" | "127.0.0.1") {
        Some(ListFormat::Hosts)
    } else if validate_domain(first).is_some() {
        // only call it Plain if the token actually looks like a domain,
        // so we don't misidentify adblock or other unsupported formats
        Some(ListFormat::Plain)
    } else {
        None
    }
}

fn parse_line(line: &str, format: ListFormat) -> Option<&str> {
    match format {
        ListFormat::Hosts => parse_hosts_line(line),
        ListFormat::Plain => parse_plain_line(line),
    }
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn validate_domain(s: &str) -> Option<&str> {
    if s.is_empty() || s.len() > 253 {
        return None;
    }

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
    parts.next()?; // skip the ip address
    let domain = parts.next()?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    validate_domain(domain)
}

fn parse_plain_line(line: &str) -> Option<&str> {
    let line = strip_comment(line).trim();
    let mut parts = line.split_ascii_whitespace();
    let domain = parts.next()?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    validate_domain(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/hosts.txt"));
    const PLAIN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plain.txt"));

    fn parse_all(content: &str) -> Vec<String> {
        let mut parser = ListParser::new();
        let mut domains = Vec::new();
        parser.push(content, |d| domains.push(d.to_owned()));
        parser.flush(|d| domains.push(d.to_owned()));
        domains
    }

    #[test]
    fn detects_hosts_format() {
        let mut parser = ListParser::new();
        parser.push(HOSTS, |_| {});
        assert!(matches!(parser.format, Some(ListFormat::Hosts)));
    }

    #[test]
    fn detects_plain_format() {
        let mut parser = ListParser::new();
        parser.push(PLAIN, |_| {});
        assert!(matches!(parser.format, Some(ListFormat::Plain)));
    }

    #[test]
    fn detects_none_for_empty() {
        let mut parser = ListParser::new();
        parser.push("# just a comment\n", |_| {});
        assert!(parser.format.is_none());
    }

    #[test]
    fn parses_hosts() {
        let domains = parse_all(HOSTS);
        assert!(domains.iter().any(|d| d == "ads.example.com"));
        assert!(domains.iter().any(|d| d == "tracker.example.com"));
        assert!(domains.iter().any(|d| d == "telemetry.example.com"));
        assert!(domains.iter().any(|d| d == "metrics.example.com"));
        assert!(domains.iter().any(|d| d == "spacing.example.com"));
        assert!(domains.iter().any(|d| d == "another.example.com"));
    }

    #[test]
    fn hosts_filters_local_domains() {
        let domains = parse_all(HOSTS);
        assert!(!domains.iter().any(|d| d == "localhost"));
        assert!(!domains.iter().any(|d| d == "localhost.localdomain"));
        assert!(!domains.iter().any(|d| d == "local"));
        assert!(!domains.iter().any(|d| d == "broadcasthost"));
        assert!(!domains.iter().any(|d| d == "ip6-localhost"));
        assert!(!domains.iter().any(|d| d == "ip6-loopback"));
    }

    #[test]
    fn parses_plain() {
        let domains = parse_all(PLAIN);
        assert!(domains.iter().any(|d| d == "ads.example.com"));
        assert!(domains.iter().any(|d| d == "tracker.example.com"));
        assert!(domains.iter().any(|d| d == "telemetry.example.com"));
        assert!(domains.iter().any(|d| d == "metrics.example.com"));
        assert!(domains.iter().any(|d| d == "*.ads.example.com"));
        assert!(domains.iter().any(|d| d == "another.example.com"));
    }

    #[test]
    fn handles_chunk_boundary_mid_line() {
        let mut parser = ListParser::new();
        let mut domains = Vec::new();
        parser.push("example", |d| domains.push(d.to_owned()));
        parser.push(".com\n", |d| domains.push(d.to_owned()));
        parser.flush(|d| domains.push(d.to_owned()));
        assert!(domains.iter().any(|d| d == "example.com"));
    }
}
