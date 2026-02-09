use std::sync::Arc;

use arc_swap::ArcSwap;
use reso_blocklist::BlocklistMatcher;
use reso_dns::domain_name::DomainName;

use crate::database::{DatabaseConnection, models::blocklist::BlockedDomain};

pub struct BlocklistService {
    matcher: ArcSwap<BlocklistMatcher>,
    connection: Arc<DatabaseConnection>,
}

impl BlocklistService {
    pub fn new(connection: Arc<DatabaseConnection>) -> Self {
        Self {
            matcher: ArcSwap::new(BlocklistMatcher::default().into()),
            connection,
        }
    }

    pub async fn add_domain(&self, domain: &str) -> anyhow::Result<()> {
        // TODO: loading a matcher on demand is expensive, we should flush them periodically like we're doing with the metrics.
        let domain = DomainName::from_user(domain)?;
        BlockedDomain::new(domain.to_string()).insert(&self.connection).await?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn remove_domain(&self, domain: &str) -> anyhow::Result<()> {
        let domain = DomainName::from_user(domain)?;

        BlockedDomain::new(domain.to_string()).delete(&self.connection).await?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn load_matcher(&self) -> anyhow::Result<()> {
        let domains = BlockedDomain::list_all(&self.connection).await?;
        let updated_matcher = BlocklistMatcher::load(domains.iter().map(|d| d.domain.as_str()))?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub fn is_blocked(&self, name: &str) -> bool {
        self.matcher.load().is_blocked(name)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod service_tests;