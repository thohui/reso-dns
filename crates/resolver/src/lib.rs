use std::sync::Arc;

use bytes::Bytes;
use once_cell::sync::OnceCell;

use async_trait::async_trait;
use reso_context::DnsRequestCtx;
use reso_dns::DnsMessage;

#[async_trait]
pub trait DnsResolver<G: Send + Sync, L> {
    async fn resolve<'a>(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Bytes>;
}

pub mod forwarder;
