use std::{net::IpAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use once_cell::sync::OnceCell;
use reso_dns::DnsMessage;
use tokio::time::Instant;

/// Classifies the kind of error that occurred during request processing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i64)]
pub enum ErrorType {
    Timeout,
    InvalidRequest,
    InvalidResponse,
    MalformedResponse,
    Other,
}

/// The type of DNS request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DnsProtocol {
    /// UDP
    UDP,
    /// TCP
    TCP,
    /// DNS over HTTPS
    DOH,
    /// DNS over TLS
    DOT,
}

/// Context for a DNS request.
/// Every request gets its own context instance.
#[derive(Debug)]
pub struct DnsRequestCtx<G, L> {
    request_address: IpAddr,
    request_type: DnsProtocol,
    raw: Bytes,
    message: OnceCell<DnsMessage>,
    budget: RequestBudget,
    global: Arc<G>,
    local: L,
}

impl<G, L> DnsRequestCtx<G, L> {
    pub fn new(
        deadline: Duration,
        request_address: IpAddr,
        request_type: DnsProtocol,
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
            local,
        }
    }

    // Request budget
    pub fn budget(&self) -> &RequestBudget {
        &self.budget
    }

    /// Request address
    pub fn request_address(&self) -> IpAddr {
        self.request_address
    }

    /// Request type
    pub fn request_type(&self) -> DnsProtocol {
        self.request_type
    }

    /// Attempt to decode and get the DNS message
    /// This also caches the decoded message for future calls.
    pub fn message(&self) -> reso_dns::Result<&DnsMessage> {
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
    pub fn local(&self) -> &L {
        &self.local
    }

    /// Mutable access to local context
    pub fn local_mut(&mut self) -> &mut L {
        &mut self.local
    }
}

pub struct DnsResponse {
    bytes: Bytes,
    message: OnceCell<DnsMessage>,
    /// The DNS protocol used that the response was resolved with.
    response_protocol: Option<DnsProtocol>,
}

impl DnsResponse {
    pub fn from_bytes(bytes: Bytes, response_protocol: Option<DnsProtocol>) -> Self {
        Self {
            bytes,
            message: OnceCell::new(),
            response_protocol,
        }
    }

    pub fn from_parsed(raw: Bytes, message: DnsMessage, response_protocol: Option<DnsProtocol>) -> Self {
        Self {
            bytes: raw,
            message: OnceCell::with_value(message),
            response_protocol,
        }
    }

    pub fn bytes(&self) -> Bytes {
        // this is fine, as `Bytes` is reference counted and cloning it is cheap.
        self.bytes.clone()
    }

    pub fn message(&self) -> reso_dns::Result<&DnsMessage> {
        self.message.get_or_try_init(|| DnsMessage::decode(&self.bytes))
    }

    pub fn response_protocol(&self) -> Option<DnsProtocol> {
        self.response_protocol
    }
}

/// Trait for DNS middlewares that can process DNS requests.
#[async_trait]
pub trait DnsMiddleware<G, L>: Send + Sync {
    /// Called when a new DNS query is received, before processing the request.
    async fn on_query(&self, _ctx: &mut DnsRequestCtx<G, L>) -> anyhow::Result<Option<DnsResponse>> {
        Ok(None)
    }
    /// Called after a response has been generated, but before it's sent to the client.
    async fn on_response(&self, _ctx: &mut DnsRequestCtx<G, L>, _response: &mut DnsResponse) -> anyhow::Result<()> {
        Ok(())
    }
    /// Called when an error occurs during request processing.
    async fn on_error(&self, _ctx: &mut DnsRequestCtx<G, L>, _error: &ErrorType, _message: &str) {}
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
