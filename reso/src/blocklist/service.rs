use arc_swap::ArcSwap;
use reso_blocklist::BlocklistMatcher;
use reso_database::DatabaseOperations;
use turso::Connection;

use super::model::BlockedDomain;

pub struct BlocklistService {
    matcher: ArcSwap<BlocklistMatcher>,
    connection: Connection,
}

impl BlocklistService {
    pub fn new(connection: Connection) -> Self {
        Self {
            matcher: ArcSwap::new(BlocklistMatcher::default().into()),
            connection,
        }
    }

    pub async fn add_domain(&self, domain: &str) -> anyhow::Result<()> {
        BlockedDomain::new(domain.to_string())
            .create(&self.connection)
            .await?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn remove_domain(&self, domain: String) -> anyhow::Result<()> {
        BlockedDomain::delete(&self.connection, &domain).await?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn load_matcher(&self) -> anyhow::Result<()> {
        let domains = BlockedDomain::all(&self.connection).await?;
        let updated_matcher = BlocklistMatcher::load(domains.iter().map(|d| d.domain()))?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub fn is_blocked(&self, name: &str) -> bool {
        self.matcher.load().is_blocked(name)
    }
}
