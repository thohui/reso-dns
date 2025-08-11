use arc_swap::ArcSwap;
use tokio::sync::Mutex;

use super::matcher::BlocklistMatcher;

pub struct BlocklistService {
    entries: Mutex<Vec<String>>, // todo: replace this with a persistent store at some point, should be fine for now. we shouldnt have to keep track of this, only lazily update it in the store.
    matcher: ArcSwap<BlocklistMatcher>,
}

impl BlocklistService {
    pub fn new() -> Self {
        Self {
            matcher: ArcSwap::new(BlocklistMatcher::new().into()),
            entries: Mutex::new(Vec::new()),
        }
    }

    pub async fn add_domain(&self, domain: &str) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().await;
        entries.push(domain.into());
        let entries: Vec<&str> = entries.iter().map(|s| s.as_str()).collect();
        let updated_matcher = BlocklistMatcher::load(entries)?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub async fn remove_domain(&self, domain: String) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().await;
        if let Some(pos) = entries.iter().position(|x| *x == domain) {
            entries.remove(pos);
            let entries: Vec<&str> = entries.iter().map(|s| s.as_str()).collect();
            let updated_matcher = BlocklistMatcher::load(entries)?;
            self.matcher.swap(updated_matcher.into());
        }
        Ok(())
    }

    pub fn is_blocked(&self, name: &str) -> bool {
        tracing::debug!("is_blocked {}", name);
        self.matcher.load().is_blocked(name)
    }
}
