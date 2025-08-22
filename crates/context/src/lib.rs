use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use reso_dns::DnsMessage;

#[derive(Debug, Clone)]
pub struct DnsRequestCtx<G, L> {
    raw: Bytes,
    message: OnceCell<DnsMessage>,
    global: Arc<G>,
    local: Arc<RwLock<L>>,
}

impl<G, L> DnsRequestCtx<G, L> {
    pub fn new(raw: Bytes, global: Arc<G>, local: L) -> Self {
        Self {
            raw,
            message: OnceCell::new(),
            global,
            local: Arc::new(RwLock::new(local)),
        }
    }

    pub fn message(&self) -> anyhow::Result<&DnsMessage> {
        self.message
            .get_or_try_init(|| DnsMessage::decode(&self.raw))
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn global(&self) -> &G {
        &self.global
    }

    pub fn local(&self) -> RwLockReadGuard<L> {
        self.local.read()
    }

    pub fn local_mut(&self) -> RwLockWriteGuard<'_, L> {
        self.local.write()
    }
}

#[async_trait]
pub trait DnsMiddleware<G, L>: Send + Sync {
    async fn on_query(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Option<Bytes>>;
}

pub async fn run_middlewares<G, L>(
    mws: std::sync::Arc<Vec<Arc<dyn DnsMiddleware<G, L>>>>,
    ctx: &DnsRequestCtx<G, L>,
) -> anyhow::Result<Option<Bytes>> {
    for m in mws.iter() {
        if let Some(resp) = m.on_query(ctx).await? {
            return Ok(Some(resp));
        }
    }
    Ok(None)
}
