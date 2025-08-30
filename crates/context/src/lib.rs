use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use reso_dns::DnsMessage;
use tokio::time::Instant;

/// The type of DNS request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    /// UDP
    UDP,
    /// TCP
    TCP,
    /// DNS over HTTPS
    DOH,
}

#[derive(Debug, Clone)]
pub struct DnsRequestCtx<G, L> {
    request_type: RequestType,
    raw: Bytes,
    message: OnceCell<DnsMessage>,
    budget: RequestBudget,
    global: Arc<G>,
    local: Arc<RwLock<L>>,
}

impl<G, L> DnsRequestCtx<G, L> {
    pub fn new(
        deadline: Duration,
        request_type: RequestType,
        raw: Bytes,
        global: Arc<G>,
        local: L,
    ) -> Self {
        Self {
            budget: RequestBudget::new(deadline),
            request_type,
            raw,
            message: OnceCell::new(),
            global,
            local: Arc::new(RwLock::new(local)),
        }
    }

    /// The deadline for the request.
    pub fn deadline(&self) -> Instant {
        self.budget.at()
    }

    // Remaining time budget for the request.
    pub fn remaining(&self) -> Option<Duration> {
        self.budget.remaining()
    }

    /// Request Type
    pub fn request_type(&self) -> RequestType {
        self.request_type
    }

    /// Lazily decode and return the DNS message.
    pub fn message(&self) -> anyhow::Result<&DnsMessage> {
        self.message
            .get_or_try_init(|| DnsMessage::decode(&self.raw))
    }

    /// Raw request bytes
    pub fn raw(&self) -> Bytes {
        self.raw.clone()
    }

    /// Global context
    pub fn global(&self) -> &G {
        &self.global
    }

    /// Local context
    pub fn local(&self) -> RwLockReadGuard<L> {
        self.local.read()
    }

    /// Mutable local context
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

/// A budget for processing a DNS request, based on a deadline.
#[derive(Debug, Clone)]
pub struct RequestBudget {
    deadline: Instant,
}

impl RequestBudget {
    pub fn new(timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + timeout,
        }
    }

    pub fn at(&self) -> Instant {
        self.deadline
    }

    pub fn remaining(&self) -> Option<Duration> {
        let now = Instant::now();
        (now < self.deadline).then_some(self.deadline - now)
    }

    pub fn cap(&self, per_step: Duration) -> Instant {
        let rem = self.remaining().unwrap_or_default();
        Instant::now() + rem.min(per_step)
    }
}
