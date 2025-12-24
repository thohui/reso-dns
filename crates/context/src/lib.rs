use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use reso_dns::DnsMessage;
use tokio::time::Instant;

/// The type of DNS request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestType {
    /// UDP
    UDP,
    /// TCP
    TCP,
    /// DNS over HTTPS
    DOH,
}

/// Context for a DNS request.
/// Every request gets its own context instance.
#[derive(Debug, Clone)]
pub struct DnsRequestCtx<G, L> {
    request_address: SocketAddr,
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
        request_address: SocketAddr,
        request_type: RequestType,
        raw: Bytes,
        global: Arc<G>,
        local: L,
    ) -> Self {
        Self {
            budget: RequestBudget::new(deadline),
            request_address,
            request_type,
            raw,
            message: OnceCell::new(),
            global,
            local: Arc::new(RwLock::new(local)),
        }
    }

    // Request budget
    pub fn budget(&self) -> &RequestBudget {
        &self.budget
    }

    /// Request address
    pub fn request_address(&self) -> &SocketAddr {
        &self.request_address
    }

    /// Request type
    pub fn request_type(&self) -> RequestType {
        self.request_type
    }

    /// Attempt to decode and get the DNS message
    /// This also caches the decoded message for future calls.
    pub fn message(&self) -> anyhow::Result<&DnsMessage> {
        self.message.get_or_try_init(|| DnsMessage::decode(&self.raw))
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

/// Trait for DNS middlewares that can process DNS requests.
#[async_trait]
pub trait DnsMiddleware<G, L>: Send + Sync {
    async fn on_query(&self, ctx: &DnsRequestCtx<G, L>) -> anyhow::Result<Option<Bytes>>;
}

/// Run the middlewares in order, returning the first response found.
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
#[derive(Debug, Clone, Copy)]
pub struct RequestBudget {
    time_created: Instant,
    deadline: Instant,
}

impl RequestBudget {
    pub fn new(timeout: Duration) -> Self {
        let now = Instant::now();
        Self {
            deadline: now + timeout,
            time_created: now,
        }
    }

    /// Elapsed time.
    pub fn elapsed(&self) -> Duration {
        let now = Instant::now();
        now - self.time_created
    }

    /// When the budget expires.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Remaining time in the budget.
    pub fn remaining(&self) -> Option<Duration> {
        let now = Instant::now();
        (now < self.deadline).then_some(self.deadline - now)
    }
}
