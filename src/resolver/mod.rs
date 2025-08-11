use std::sync::Arc;

use arc_swap::Cache;
use once_cell::sync::OnceCell;

use async_trait::async_trait;
use bytes::Bytes;

use crate::{dns::message::DnsMessage, services::Services};

pub mod forwarder;

pub struct DnsRequestCtx<'a> {
    raw: &'a [u8],
    message: OnceCell<DnsMessage>,
    services: Arc<Services>,
}

impl<'a> DnsRequestCtx<'a> {
    pub fn new(raw: &'a [u8], services: Arc<Services>) -> Self {
        Self {
            raw,
            message: OnceCell::new(),
            services,
        }
    }

    pub fn message(&self) -> anyhow::Result<&DnsMessage> {
        self.message
            .get_or_try_init(|| DnsMessage::decode(self.raw))
    }

    pub fn raw(&self) -> &[u8] {
        self.raw
    }

    pub fn services(&self) -> &Services {
        &self.services
    }
}

#[async_trait]
pub trait DnsResolver: Send + Sync {
    async fn resolve(&self, ctx: &DnsRequestCtx) -> anyhow::Result<Bytes>;
}
