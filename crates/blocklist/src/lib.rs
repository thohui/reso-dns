/// Node in the trie structure, representing a blocklist entry.
#[derive(Debug, Clone, Default)]
struct Node {
    wildcard: bool,
    blocked: bool,
    children: Vec<(String, Node)>,
}

impl Node {
    fn child_mut(&mut self, label: &str) -> &mut Node {
        match self
            .children
            .binary_search_by(|(l, _)| l.as_str().cmp(label))
        {
            Ok(i) => &mut self.children[i].1,
            Err(i) => {
                self.children
                    .insert(i, (label.to_string(), Node::default()));
                &mut self.children[i].1
            }
        }
    }
}

/// Trie implementation of a blocklist matcher.
#[derive(Debug, Clone, Default)]
pub struct BlocklistMatcher {
    root: Node,
}

impl BlocklistMatcher {
    /// Check if a given domain name is blocked.
    pub fn is_blocked(&self, name: &str) -> bool {
        let labels = match normalize_to_rev_labels(name) {
            Ok(labels) => labels,
            Err(_) => return false,
        };

        let mut node = &self.root;

        for label in labels {
            if node.wildcard {
                return true;
            }

            match node
                .children
                .binary_search_by(|(l, _)| l.as_str().cmp(&label))
            {
                Ok(i) => node = &node.children[i].1,
                Err(_) => return false,
            }
        }

        node.blocked
    }

    /// Load blocklist patterns from an iterator of strings.
    pub fn load<'a, I>(patterns: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut root = Node::default();

        for pat in patterns {
            let pat = pat.trim();
            if pat.is_empty() {
                continue;
            }

            let (is_wildcard, name) = if let Some(rest) = pat.strip_prefix("*.") {
                (true, rest)
            } else {
                (false, pat)
            };

            let labels = normalize_to_rev_labels(name)?;
            if labels.is_empty() {
                continue;
            }

            let mut node = &mut root;
            for label in labels {
                node = node.child_mut(&label);
            }

            if is_wildcard {
                node.wildcard = true;
            } else {
                node.blocked = true;
            }
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
        let patterns: Vec<&str> = vec!["google.com", "yahoo.com", "*.bla.com"];
        let matcher = BlocklistMatcher::load(patterns).unwrap();
        assert!(matcher.is_blocked("google.com"));
        assert!(matcher.is_blocked("yahoo.com"));
        assert!(matcher.is_blocked("a.bla.com"));
    }
}
