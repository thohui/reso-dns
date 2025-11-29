use bytes::Bytes;

use async_trait::async_trait;
use reso_context::DnsRequestCtx;
use reso_dns::DnsResponseCode;
use thiserror::Error;

/// Trait for DNS resolvers that can resolve DNS requests.
#[async_trait]
pub trait DnsResolver<G: Send + Sync, L> {
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> Result<Bytes, ResolveError>;
}

/// Error type for DNS resolvers
#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("request timed out")]
    Timeout,

    #[error("failed to decode incoming request: {0}")]
    Decode(#[source] anyhow::Error),

    #[error("invalid response {0}")]
    InvalidResponse(String),

    #[error("unexpected error {0}")]
    Other(#[source] anyhow::Error),
}

impl ResolveError {
    pub fn response_code(&self) -> DnsResponseCode {
        match self {
            ResolveError::Timeout => DnsResponseCode::ServerFailure,
            ResolveError::Decode(_) => DnsResponseCode::FormatError,
            ResolveError::InvalidResponse(_) => DnsResponseCode::ServerFailure,
            ResolveError::Other(_) => DnsResponseCode::ServerFailure,
        }
    }
}

pub mod forwarder;
