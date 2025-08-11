use std::sync::Arc;

use arc_swap::Cache;
use once_cell::sync::OnceCell;

use async_trait::async_trait;
use bytes::Bytes;

use crate::{cache::service::CacheService, dns::message::DnsMessage};

pub mod forwarder;

pub struct DnsRequestCtx<'a> {
    raw: &'a [u8],
    message: OnceCell<DnsMessage>,
    cache_service: Arc<CacheService>,
}

impl<'a> DnsRequestCtx<'a> {
    pub fn new(raw: &'a [u8], cache_service: Arc<CacheService>) -> Self {
        Self {
            raw,
            message: OnceCell::new(),
            cache_service,
        }
    }

    pub fn message(&self) -> anyhow::Result<&DnsMessage> {
        self.message
            .get_or_try_init(|| DnsMessage::decode(self.raw))
    }

    pub fn raw(&self) -> &[u8] {
        self.raw
    }

    pub fn cache_service(&self) -> Arc<CacheService> {
        self.cache_service.clone()
    }
}

#[async_trait]
pub trait DnsResolver: Send + Sync {
    async fn resolve(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Bytes>;
}
