use std::sync::Arc;

use arc_swap::ArcSwap;
use reso_blocklist::BlocklistMatcher;
use reso_dns::domain_name::DomainName;

use crate::database::{CoreDatabasePool, models::blocked_domain::BlockedDomain};

pub struct BlocklistService {
    matcher: ArcSwap<BlocklistMatcher>,
    connection: Arc<CoreDatabasePool>,
}

impl BlocklistService {
    pub async fn initialize(connection: Arc<CoreDatabasePool>) -> anyhow::Result<Self> {
        let domains = BlockedDomain::list_all(&connection).await?;
        let matcher = BlocklistMatcher::load(domains.iter().filter(|d| d.enabled).map(|d| d.domain.as_str()))?;

        Ok(Self {
            matcher: ArcSwap::new(matcher.into()),
            connection,
        })
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

    pub async fn toggle_domain(&self, domain: &str) -> anyhow::Result<()> {
        let domain = DomainName::from_user(domain)?;
        BlockedDomain::toggle(&domain.to_string(), &self.connection).await?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn load_matcher(&self) -> anyhow::Result<()> {
        let domains = BlockedDomain::list_all(&self.connection).await?;
        let updated_matcher = BlocklistMatcher::load(domains.iter().filter(|d| d.enabled).map(|d| d.domain.as_str()))?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub fn is_blocked(&self, name: &str) -> bool {
        self.matcher.load().is_blocked(name)
    }
}
