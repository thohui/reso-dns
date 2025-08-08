use std::collections::HashMap;

/// Blocklist matcher
pub trait Matcher: Send + Sync {
    /// Check if a domain is blocked.
    fn is_blocked(&self, name: &str) -> bool;
    /// Load the pattenrs into the matcher.
    fn load<'a, I>(patterns: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = &'a str>,
        Self: Sized;
}

#[derive(Default)]
struct Node {
    blocked: bool,
    children: HashMap<String, Node>,
}
// Trie implementation of a blocklist matcher.
pub struct TrieMatcher {
    root: Node,
}

impl Matcher for TrieMatcher {
    fn is_blocked(&self, name: &str) -> bool {
        let labels = normalize_to_rev_labels(name).unwrap_or_default();

        let mut node = &self.root;
        for label in &labels {
            if let Some(next) = node.children.get(label) {
                if next.blocked {
                    return true;
                }
                node = next;
                continue;
            }
            if let Some(wild) = node.children.get("*") {
                if wild.blocked {
                    return true;
                }
                node = wild;
                continue;
            }
            return false;
        }
        node.blocked
    }

    fn load<'a, I>(patterns: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = &'a str>,
        Self: Sized,
    {
        let mut root = Node::default();

        for pattern in patterns {
            let labels = normalize_to_rev_labels(&pattern)?;
            if labels.is_empty() {
                continue;
            }

            let mut node = &mut root;

            for label in labels {
                node = node.children.entry(label).or_default();
            }
            node.blocked = true;
        }

        Ok(Self { root })
    }
}

/// Normalize to reverse labels:
/// "Ads.Example.COM." -> ["com","example","ads"]
/// "*.example.com"    -> ["com","example","*"]
fn normalize_to_rev_labels(input: &str) -> anyhow::Result<Vec<String>> {
    let s = input.trim().trim_end_matches('.').to_ascii_lowercase();

    // Convert Unicode to ASCII.
    let ascii =
        idna::domain_to_ascii(&s).map_err(|_| anyhow::anyhow!("invalid domain: {}", input))?;

    // Split labels, map "*" to itself, reject empties (except root)
    let mut labels: Vec<String> = ascii
        .split('.')
        .map(|l| {
            if l == "*" {
                "*".to_string()
            } else {
                l.to_string()
            }
        })
        .filter(|l| !l.is_empty())
        .collect();

    // Reverse for suffix matching
    labels.reverse();
    Ok(labels)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn test_blocked_patterns() {
        let patterns: Vec<&str> = vec!["google.com".into(), "yahoo.com".into(), "*.bla.com".into()];
        let matcher = TrieMatcher::load(patterns).unwrap();
        assert!(matcher.is_blocked("google.com".into()));
        assert!(matcher.is_blocked("yahoo.com"));
        assert!(matcher.is_blocked("a.bla.com"));
    }
}
