use std::sync::Arc;

use arc_swap::ArcSwap;
use reso_blocklist::BlocklistMatcher;
use reso_dns::domain_name::DomainName;

use crate::database::{CoreDatabasePool, models::blocked_domain::BlockedDomain};

use super::ServiceError;

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

    pub async fn add_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain =
            DomainName::from_user(domain).map_err(|e| ServiceError::BadRequest(format!("Invalid domain: {e}")))?;

        let model = BlockedDomain::new(domain.to_string());

        model.insert(&self.connection).await.map_err(|e| {
            if e.is_unique_constraint_violation() {
                ServiceError::Conflict("Domain is already blocked.".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;

        self.load_matcher().await?;
        Ok(())
    }

    pub async fn remove_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain =
            DomainName::from_user(domain).map_err(|e| ServiceError::BadRequest(format!("Invalid domain: {e}")))?;

        BlockedDomain::new(domain.to_string())
            .delete(&self.connection)
            .await
            .map_err(|e| ServiceError::Internal(e.into()))?;
        self.load_matcher().await?;
        Ok(())
    }

    pub async fn toggle_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain =
            DomainName::from_user(domain).map_err(|e| ServiceError::BadRequest(format!("Invalid domain: {e}")))?;

        let changed = BlockedDomain::toggle(&domain.to_string(), &self.connection)
            .await
            .map_err(|e| ServiceError::Internal(e.into()))?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.load_matcher().await?;
        Ok(())
    }

    pub async fn load_matcher(&self) -> Result<(), ServiceError> {
        let domains = BlockedDomain::list_all(&self.connection)
            .await
            .map_err(|e| ServiceError::Internal(e.into()))?;

        let updated_matcher = BlocklistMatcher::load(domains.iter().filter(|d| d.enabled).map(|d| d.domain.as_str()))
            .map_err(|e| ServiceError::Internal(e.into()))?;
        self.matcher.swap(updated_matcher.into());
        Ok(())
    }

    pub fn is_blocked(&self, name: &str) -> bool {
        self.matcher.load().is_blocked(name)
    }
}
