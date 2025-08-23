use bytes::Bytes;

use async_trait::async_trait;
use reso_context::DnsRequestCtx;

#[async_trait]
pub trait DnsResolver<G: Send + Sync, L> {
    async fn resolve<'a>(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes>;
}

pub mod forwarder;
