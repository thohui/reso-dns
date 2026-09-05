use std::num::NonZeroU32;

pub mod parser;

#[derive(Debug, Clone)]
pub enum DomainPattern<'a> {
    /// Matches exactly this domain.
    Exact(&'a str),
    /// Matches any subdomain of this domain but not the domain itself
    Subdomain(&'a str),
    /// Matches this domain and all its subdomains
    Domain(&'a str),
}

/// Node in the trie structure, representing a domain list entry.
#[derive(Debug, Clone, Default)]
struct Node {
    label: smol_str::SmolStr,
    /// Any further labels still match
    subdomain_match: Option<NonZeroU32>,
    /// Stopping here is a valid match
    pattern_end: Option<NonZeroU32>,
    /// Children, sorted by label for efficient lookup
    children: Vec<Node>,
}

impl Node {
    fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            subdomain_match: None,
            pattern_end: None,
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
#[derive(Debug, Clone)]
pub struct DomainListMatcher<T> {
    root: Node,
    data: Vec<T>,
}

impl<T> DomainListMatcher<T> {
    /// Check if a given domain matches any of the domain list patterns.
    pub fn exists(&self, name: &str) -> Option<&T> {
        let labels = match normalize(name) {
            Ok(labels) => labels,
            Err(_) => return None,
        };

        let mut node = &self.root;

        // the most specific candidate wins, exact -> deeper wildcard -> shallower wildcard
        let mut fallback = None;

        for label in labels.rev_labels() {
            if node.subdomain_match.is_some() {
                fallback = node.subdomain_match;
            }

            match node.children.binary_search_by(|n| n.label.as_str().cmp(label)) {
                Ok(i) => node = &node.children[i],
                Err(_) => return fallback.and_then(|index| self.get(index)),
            }
        }

        node.pattern_end.or(fallback).and_then(|index| self.get(index))
    }

    fn get(&self, index: NonZeroU32) -> Option<&T> {
        self.data.get(index.get() as usize - 1)
    }

    /// Load a list of domain patterns into the matcher.
    pub fn load<'a>(patterns: impl IntoIterator<Item = (DomainPattern<'a>, T)>) -> anyhow::Result<Self> {
        let mut root = Node::default();

        let mut entries: Vec<T> = Vec::new();

        for (pat, data) in patterns {
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

            entries.push(data);

            let index = u32::try_from(entries.len())
                .ok()
                .and_then(NonZeroU32::new)
                .ok_or_else(|| anyhow::anyhow!("domain list exceeds {} patterns", u32::MAX))?;

            if pattern_end {
                node.pattern_end = Some(index);
            }
            if subdomain_match {
                node.subdomain_match = Some(index);
            }
        }

        root.shrink();

        Ok(Self { root, data: entries })
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
            (DomainPattern::Exact("ads.google.com"), 1),
            (DomainPattern::Exact("google.com"), 2),
            (DomainPattern::Exact("yahoo.com"), 3),
            (DomainPattern::Subdomain("bla.com"), 4),
        ];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert_eq!(matcher.exists("ads.google.com"), Some(&1));
        assert_eq!(matcher.exists("google.com"), Some(&2));
        assert_eq!(matcher.exists("yahoo.com"), Some(&3));
        assert_eq!(matcher.exists("a.bla.com"), Some(&4));
        assert_eq!(matcher.exists("com"), None);
    }

    #[test]
    fn test_normalization() {
        let patterns = vec![
            (DomainPattern::Subdomain("  Example.COM.  "), 1),
            (DomainPattern::Exact("foo.bar.com"), 2),
        ];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert_eq!(matcher.exists("a.example.com"), Some(&1));
        assert_eq!(matcher.exists("foo.bar.com"), Some(&2));
        assert_eq!(matcher.exists("example.com"), None);
    }

    #[test]
    fn test_domain_pattern_matches_domain_and_subdomains() {
        let patterns = vec![(DomainPattern::Domain("example.com"), 1)];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert_eq!(matcher.exists("example.com"), Some(&1));
        assert_eq!(matcher.exists("sub.example.com"), Some(&1));
        assert_eq!(matcher.exists("deep.sub.example.com"), Some(&1));
        assert_eq!(matcher.exists("notexample.com"), None);

        // an exact and a subdomain pattern share a node without overriding each other.
        let patterns = vec![
            (DomainPattern::Exact("example.com"), 1),
            (DomainPattern::Subdomain("example.com"), 2),
        ];
        let matcher = DomainListMatcher::load(patterns).unwrap();
        assert_eq!(matcher.exists("example.com"), Some(&1));
        assert_eq!(matcher.exists("x.example.com"), Some(&2));
    }
}
