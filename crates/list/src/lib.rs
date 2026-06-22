pub mod parser;

#[derive(Debug, Clone)]
pub enum DomainPattern<'a> {
    /// Matches exactly this domain.
    Exact(&'a str),
    /// Matches any subdomain of this domain but not the domain itself (e.g. `*.example.com`).
    Subdomain(&'a str),
    /// Matches this domain and all its subdomains (adblock `||domain^` semantics).
    Domain(&'a str),
}

/// Node in the trie structure, representing a domain list entry.
#[derive(Debug, Clone, Default)]
struct Node {
    label: smol_str::SmolStr,
    /// Any further labels still match
    subdomain_match: bool,
    /// Stopping here is a valid match
    pattern_end: bool,
    /// Children, sorted by label for efficient lookup
    children: Vec<Node>,
}

impl Node {
    fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            subdomain_match: false,
            pattern_end: false,
            children: Vec::new(),
        }
    }
    fn child_mut(&mut self, label: &str) -> &mut Node {
        match self.children.binary_search_by(|l| l.label.as_str().cmp(label)) {
            Ok(i) => &mut self.children[i],
            Err(i) => {
                self.children.insert(i, Node::new(label));
                &mut self.children[i]
            }
        }
    }

    fn shrink(&mut self) {
        self.children.shrink_to_fit();
        for node in &mut self.children {
            node.shrink();
        }
    }
}

/// Trie implementation of a domain list matcher. Used for allowlists and blocklists.
/// The nodes are sorted to allow binary search for child nodes.
#[derive(Debug, Clone, Default)]
pub struct DomainListMatcher {
    root: Node,
}

impl DomainListMatcher {
    /// Check if a given domain matches any of the domain list patterns.
    pub fn exists(&self, name: &str) -> bool {
        let labels = match normalize(name) {
            Ok(labels) => labels,
            Err(_) => return false,
        };

        let mut node = &self.root;

        for label in labels.rev_labels() {
            if node.subdomain_match {
                return true;
            }

            match node.children.binary_search_by(|n| n.label.as_str().cmp(label)) {
                Ok(i) => node = &node.children[i],
                Err(_) => return false,
            }
        }

        node.pattern_end
    }

    /// Load a list of domain patterns into the matcher.
    pub fn load<'a>(patterns: impl IntoIterator<Item = DomainPattern<'a>>) -> anyhow::Result<Self> {
        let mut root = Node::default();

        for pat in patterns {
            let (name, pattern_end, subdomain_match) = match pat {
                DomainPattern::Exact(s) => (s, true, false),
                DomainPattern::Subdomain(s) => (s, false, true),
                DomainPattern::Domain(s) => (s, true, true),
            };

            let name = name.trim();
            if name.is_empty() {
                continue;
            }

            let labels = normalize(name)?;
            if labels.0.is_empty() {
                continue;
            }

            let mut node = &mut root;
            for label in labels.rev_labels() {
                node = node.child_mut(label);
            }

            if pattern_end {
                node.pattern_end = true;
            }
            if subdomain_match {
                node.subdomain_match = true;
            }
        }

        root.shrink();

        Ok(Self { root })
    }
}

pub struct NormalizedDomain(String);

impl NormalizedDomain {
    fn rev_labels(&self) -> impl Iterator<Item = &str> {
        self.0.split('.').filter(|l| !l.is_empty()).rev()
    }
}

/// Normalize a domain name using IDNA.
fn normalize(input: &str) -> anyhow::Result<NormalizedDomain> {
    let s = input.trim().trim_end_matches('.');

    // Convert Unicode to ASCII.
    let ascii = idna::domain_to_ascii(s).map_err(|_| anyhow::anyhow!("invalid domain: {}", input))?;

    Ok(NormalizedDomain(ascii))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn test_patterns() {
        let patterns = vec![
            DomainPattern::Exact("google.com"),
            DomainPattern::Exact("yahoo.com"),
            DomainPattern::Subdomain("bla.com"),
        ];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert!(matcher.exists("google.com"));
        assert!(matcher.exists("yahoo.com"));
        assert!(matcher.exists("a.bla.com"));
    }

    #[test]
    fn test_normalization() {
        let patterns = vec![
            DomainPattern::Subdomain("  Example.COM.  "),
            DomainPattern::Exact("foo.bar.com"),
        ];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert!(matcher.exists("a.example.com"));
        assert!(matcher.exists("foo.bar.com"));
        assert!(!matcher.exists("example.com"));
    }

    #[test]
    fn test_domain_pattern_matches_domain_and_subdomains() {
        let patterns = vec![DomainPattern::Domain("example.com")];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert!(matcher.exists("example.com"));
        assert!(matcher.exists("sub.example.com"));
        assert!(matcher.exists("deep.sub.example.com"));
        assert!(!matcher.exists("notexample.com"));
    }
}
