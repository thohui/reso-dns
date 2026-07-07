use crate::DomainPattern;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ListFormat {
    /// Hosts file format (https://deepwiki.com/hagezi/dns-blocklists/5.3-hosts-format)
    Hosts,
    /// Plain (domains) format (https://deepwiki.com/hagezi/dns-blocklists/5.2-domains-format)
    Plain,
    /// Adblock format (https://deepwiki.com/hagezi/dns-blocklists/5.5-adblock-format)
    Adblock,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RuleType {
    Allow,
    Block,
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
    pub fn push<F: FnMut((DomainPattern<'_>, RuleType))>(&mut self, chunk: &str, mut callback: F) {
        self.leftover.push_str(chunk);

        let mut start = 0;
        while let Some(rel_pos) = self.leftover[start..].find('\n') {
            let end = start + rel_pos;
            let line = self.leftover[start..end].trim_end_matches('\r');

            // detect format from first non comment or non empty line
            if self.format.is_none() {
                self.format = detect_line_format(line);
            }
            if let Some(fmt) = self.format {
                parse_line(line, fmt, &mut callback);
            }

            start = end + 1;
        }

        // keep the incomplete line for the next chunk
        self.leftover.drain(..start);
    }

    /// Process any remaining incomplete line after the last chunk.
    pub fn flush<F: FnMut((DomainPattern<'_>, RuleType))>(mut self, mut callback: F) {
        if !self.leftover.is_empty() {
            // file with no trailing newline
            let leftover = self.leftover;
            let line = leftover.trim_end_matches('\r');
            if self.format.is_none() {
                self.format = detect_line_format(line);
            }
            if let Some(fmt) = self.format {
                parse_line(line, fmt, &mut callback);
            }
        }
    }
}

fn detect_line_format(line: &str) -> Option<ListFormat> {
    let line = line.trim();

    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    if line.starts_with('!') || line.starts_with("[Adblock") {
        return Some(ListFormat::Adblock);
    }

    let mut parts = line.split_ascii_whitespace();
    let first = parts.next()?;
    if first.parse::<std::net::IpAddr>().is_ok() {
        Some(ListFormat::Hosts)
    } else if (first.starts_with("||") || first.starts_with("@@")) && first.contains('^') {
        Some(ListFormat::Adblock)
    } else if validate_domain(first).is_some() {
        // only call it Plain if the token actually looks like a domain,
        // so we don't misidentify adblock or other unsupported formats
        Some(ListFormat::Plain)
    } else {
        None
    }
}

fn parse_line<'a, F: FnMut((DomainPattern<'a>, RuleType))>(line: &'a str, format: ListFormat, callback: &mut F) {
    match format {
        ListFormat::Hosts => parse_hosts_line(line, callback),
        ListFormat::Plain => {
            if let Some(pat) = parse_plain_line(line) {
                callback((pat, RuleType::Block));
            }
        }
        ListFormat::Adblock => {
            if let Some(entry) = parse_adblock_line(line) {
                callback(entry);
            }
        }
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

fn parse_hosts_line<'a, F: FnMut((DomainPattern<'a>, RuleType))>(line: &'a str, callback: &mut F) {
    let line = strip_comment(line).trim();
    if line.is_empty() {
        return;
    }
    let mut parts = line.split_ascii_whitespace();
    parts.next(); // skip the ip address

    // compressed hosts lines can list multiple domains after the ip
    for domain in parts {
        if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
            continue;
        }
        if let Some(domain) = validate_domain(domain) {
            callback((DomainPattern::Exact(domain), RuleType::Block));
        }
    }
}

fn parse_plain_line(line: &str) -> Option<DomainPattern<'_>> {
    let line = strip_comment(line).trim();
    let mut parts = line.split_ascii_whitespace();
    let domain = parts.next()?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    let domain = validate_domain(domain)?;
    if let Some(rest) = domain.strip_prefix("*.") {
        Some(DomainPattern::Subdomain(rest))
    } else {
        Some(DomainPattern::Domain(domain))
    }
}

fn parse_adblock_line(line: &str) -> Option<(DomainPattern<'_>, RuleType)> {
    let line = line.trim();

    // AdblockPlus uses ! for comments and [ for metadata headers like [Adblock Plus 2.0]
    if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
        return None;
    }

    // Check for exception (allowlist) rule: @@||domain^
    let (line, rule_type) = if let Some(rest) = line.strip_prefix("@@") {
        (rest, RuleType::Allow)
    } else {
        (line, RuleType::Block)
    };

    let rest = line.strip_prefix("||")?;
    let domain = rest.split_once('^').map(|(d, _)| d)?;

    // skip rules with path components
    if domain.contains('/') {
        return None;
    }

    let domain = validate_domain(domain)?;

    if LOCAL_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
        return None;
    }

    let pattern = if let Some(base) = domain.strip_prefix("*.") {
        DomainPattern::Subdomain(base)
    } else {
        DomainPattern::Domain(domain)
    };
    Some((pattern, rule_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/hosts.txt"));
    const PLAIN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plain.txt"));
    const ADBLOCK: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/adblock.txt"));

    #[derive(Debug, PartialEq)]
    enum PatternKind {
        Exact,
        Subdomain,
        Domain,
    }

    fn parse_all(content: &str) -> Vec<(String, RuleType, PatternKind)> {
        let mut parser = ListParser::new();
        let mut domains = Vec::new();
        parser.push(content, |(pat, rule_type)| {
            let (s, kind) = match pat {
                DomainPattern::Exact(s) => (s.to_owned(), PatternKind::Exact),
                DomainPattern::Subdomain(s) => (s.to_owned(), PatternKind::Subdomain),
                DomainPattern::Domain(s) => (s.to_owned(), PatternKind::Domain),
            };
            domains.push((s, rule_type, kind));
        });
        parser.flush(|(pat, rule_type)| {
            let (s, kind) = match pat {
                DomainPattern::Exact(s) => (s.to_owned(), PatternKind::Exact),
                DomainPattern::Subdomain(s) => (s.to_owned(), PatternKind::Subdomain),
                DomainPattern::Domain(s) => (s.to_owned(), PatternKind::Domain),
            };
            domains.push((s, rule_type, kind));
        });
        domains
    }

    fn cmp_block((domain, rule_type, _): &(String, RuleType, PatternKind), expected: &str) -> bool {
        domain == expected && *rule_type == RuleType::Block
    }

    fn cmp_allow((domain, rule_type, _): &(String, RuleType, PatternKind), expected: &str) -> bool {
        domain == expected && *rule_type == RuleType::Allow
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
        assert!(domains.iter().any(|d| cmp_block(d, "ads.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "tracker.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "telemetry.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "metrics.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "spacing.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "another.example.com")));
    }

    #[test]
    fn parses_compressed_hosts_lines() {
        let domains = parse_all(HOSTS);
        assert!(domains.iter().any(|d| cmp_block(d, "multi1.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "multi2.example.com")));
        // local domains are filtered even when mixed into a compressed line
        assert!(domains.iter().any(|d| cmp_block(d, "multi3.example.com")));
        assert!(!domains.iter().any(|d| d.0 == "localhost"));
    }

    #[test]
    fn hosts_filters_local_domains() {
        let domains = parse_all(HOSTS);
        assert!(!domains.iter().any(|d| cmp_block(d, "localhost")));
        assert!(!domains.iter().any(|d| cmp_block(d, "localhost.localdomain")));
        assert!(!domains.iter().any(|d| cmp_block(d, "local")));
        assert!(!domains.iter().any(|d| cmp_block(d, "broadcasthost")));
        assert!(!domains.iter().any(|d| cmp_block(d, "ip6-localhost")));
        assert!(!domains.iter().any(|d| cmp_block(d, "ip6-loopback")));
    }

    #[test]
    fn parses_plain() {
        let domains = parse_all(PLAIN);
        assert!(domains.iter().any(|d| cmp_block(d, "ads.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "tracker.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "telemetry.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "metrics.example.com")));
        assert!(domains
            .iter()
            .any(|(d, rt, kind)| d == "ads.example.com" && *rt == RuleType::Block && *kind == PatternKind::Subdomain));
        assert!(domains.iter().any(|d| cmp_block(d, "another.example.com")));
    }

    #[test]
    fn detects_adblock_format() {
        let mut parser = ListParser::new();
        parser.push(ADBLOCK, |_| {});
        assert!(matches!(parser.format, Some(ListFormat::Adblock)));
    }

    #[test]
    fn detects_adblock_from_header_before_rules() {
        let mut parser = ListParser::new();
        parser.push("! Title: My List\n", |_| {});
        assert!(matches!(parser.format, Some(ListFormat::Adblock)));

        let mut parser = ListParser::new();
        parser.push("[Adblock Plus 2.0]\n", |_| {});
        assert!(matches!(parser.format, Some(ListFormat::Adblock)));
    }

    #[test]
    fn parses_adblock_block_rules() {
        let domains = parse_all(ADBLOCK);
        assert!(domains.iter().any(|d| cmp_block(d, "ads.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "tracker.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "telemetry.example.com")));
        assert!(domains.iter().any(|d| cmp_block(d, "metrics.example.com")));
    }

    #[test]
    fn parses_adblock_allow_rules() {
        let domains = parse_all(ADBLOCK);
        assert!(domains.iter().any(|d| cmp_allow(d, "safe.example.com")));
        assert!(domains.iter().any(|d| cmp_allow(d, "cdn.example.com")));
    }

    #[test]
    fn adblock_ignores_non_domain_rules() {
        let domains = parse_all(ADBLOCK);
        assert!(!domains.iter().any(|d| d.0 == "example.com"));
        assert!(!domains.iter().any(|d| d.0.contains('/')));
    }

    #[test]
    fn handles_chunk_boundary_mid_line() {
        let mut parser = ListParser::new();
        let mut domains = Vec::new();
        parser.push("example", |(pat, rt)| {
            if let DomainPattern::Domain(s) = pat {
                domains.push((s.to_owned(), rt, PatternKind::Domain));
            }
        });
        parser.push(".com\n", |(pat, rt)| {
            if let DomainPattern::Domain(s) = pat {
                domains.push((s.to_owned(), rt, PatternKind::Domain));
            }
        });
        parser.flush(|(pat, rt)| {
            if let DomainPattern::Domain(s) = pat {
                domains.push((s.to_owned(), rt, PatternKind::Domain));
            }
        });
        assert!(domains.iter().any(|d| cmp_block(d, "example.com")));
    }
}
