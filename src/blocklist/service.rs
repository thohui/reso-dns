use arc_swap::ArcSwap;

use super::matcher::Matcher;

pub struct BlocklistService<M: Matcher> {
    entries: Vec<String>, // todo: replace this with a persistent store at some point, should be fine for now. we shouldnt have to keep track of this, only lazily update it in the store.
    matcher: ArcSwap<M>,
}

impl<M: Matcher> BlocklistService<M> {
    pub fn new(matcher: M) -> Self {
        Self {
            matcher: ArcSwap::new(matcher.into()),
            entries: Vec::new(),
        }
    }

    pub fn add_domain(&mut self, domain: &str) -> anyhow::Result<()> {
        self.entries.push(domain.into());
        let entries: Vec<&str> = self.entries.iter().map(|s| s.as_str()).collect();
        let updated_matcher = M::load(entries)?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub fn remove_domain(&mut self, domain: String) -> anyhow::Result<()> {
        if let Some(pos) = self.entries.iter().position(|x| *x == domain) {
            self.entries.remove(pos);
            let entries: Vec<&str> = self.entries.iter().map(|s| s.as_str()).collect();
            let updated_matcher = M::load(entries)?;
            self.matcher.swap(updated_matcher.into());
        }
        Ok(())
    }
}
