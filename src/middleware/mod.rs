use async_trait::async_trait;
use bytes::Bytes;

use crate::resolver::DnsRequestCtx;

pub mod blocklist;
pub mod cache;

#[async_trait]
pub trait DnsMiddleware: Send + Sync {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>>;
}
