use async_trait::async_trait;
use bytes::Bytes;

use crate::resolver::DnsRequestCtx;

pub mod cache;

#[async_trait]
pub trait DnsMiddleware: Send + Sync {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>>;
}

pub struct TestMiddleware;

#[async_trait]
impl DnsMiddleware for TestMiddleware {
    async fn on_query(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Option<Bytes>> {
        println!("testing {}", ctx.message().unwrap().id);
        Ok(None)
    }
}
