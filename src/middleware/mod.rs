use async_trait::async_trait;
use bytes::Bytes;

#[async_trait]
pub trait DnsMiddleware: Send + Sync {
    async fn on_query(&self, packet: &[u8]) -> anyhow::Result<Option<Bytes>>;
}
