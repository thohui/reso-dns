use async_trait::async_trait;
use bytes::Bytes;

pub mod forwarder;

#[async_trait]
pub trait DnsResolver: Send + Sync {
    async fn resolve(&self, query: &[u8]) -> anyhow::Result<Bytes>;
}
