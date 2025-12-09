use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use rand::Rng;
use reso_cache::CacheKey;
use reso_context::DnsRequestCtx;
use reso_dns::DnsMessage;
use reso_inflight::Inflight;

use crate::{DnsResolver, ResolveError};

use super::{
    request::UpstreamResolveRequest,
    upstream::{Limits, Upstreams},
};

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstreams: Arc<Upstreams>,
    inflight_requests: Inflight<CacheKey, DnsResponseBytes>,
}

impl ForwardResolver {
    pub async fn new(upstreams: &[SocketAddr]) -> anyhow::Result<Self> {
        if upstreams.is_empty() {
            tracing::warn!(
                "No upstreams configured for forward resolver, it will not be able to resolve any queries!"
            );
        }
        Ok(Self {
            upstreams: Arc::new(
                Upstreams::new(
                    upstreams,
                    // TODO: make this configurable
                    Limits {
                        connect_timeout: Duration::from_secs(5),
                        max_tcp_connections: 100,
                        max_idle_tcp_connections: 100,
                        tcp_ttl: Duration::from_secs(30),
                    },
                )
                .await?,
            ),
            inflight_requests: Inflight::new(),
        })
    }
}

#[async_trait]
impl<G, L> DnsResolver<G, L> for ForwardResolver
where
    G: Send + Sync + 'static,
    L: Send + Sync,
{
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> Result<Bytes, ResolveError> {
        let query_message = ctx
            .message()
            .or_else(|e| Err(ResolveError::InvalidRequest(e.to_string())))?;

        if query_message.questions().len() != 1 {
            return Err(ResolveError::InvalidRequest(format!(
                "request contains {} questions, expected 1",
                query_message.questions().len(),
            )));
        }

        let key = CacheKey::try_from(query_message).or_else(|e| Err(ResolveError::Other(e)))?;

        let upstreams = self.upstreams.clone();

        let query = ctx.raw();
        let request_type = ctx.request_type();
        let budget = ctx.budget().clone();

        let resp_arc = self
            .inflight_requests
            .get_or_run(key, async move |_| {
                let (randomized_query, _) = generate_tid(&query);
                let request =
                    UpstreamResolveRequest::new(request_type, randomized_query, budget, upstreams);
                let response = request.resolve().await?;
                Ok(DnsResponseBytes::new(response))
            })
            .await
            .map_err(|e| match e.downcast::<ResolveError>() {
                Ok(e) => e,
                Err(e) => ResolveError::Other(e),
            })?;

        let response = resp_arc
            .as_ref()
            .clone()
            .into_custom_response(query_message.id);

        let response_message = DnsMessage::decode(&response)
            .or_else(|e| Err(ResolveError::InvalidResponse(e.to_string())))?;

        // ensure that the response has exactly one question
        if response_message.questions().len() != 1 {
            return Err(ResolveError::InvalidResponse(std::format!(
                "upstream response contains {} questions, expected 1",
                response_message.questions().len(),
            )));
        }

        let req_q = query_message.questions().first();
        let resp_q = response_message.questions().first();

        // ensure that the response question matches the request question
        if req_q != resp_q {
            return Err(ResolveError::InvalidResponse(
                "upstream response question does not match request question".to_string(),
            ));
        }
        Ok(response)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct DnsResponseBytes(Bytes);

impl DnsResponseBytes {
    pub fn new(bytes: Bytes) -> Self {
        Self(bytes)
    }

    pub fn into_custom_response(self, transaction_id: u16) -> Bytes {
        let mut bytes = BytesMut::from(&self.0[0..]);
        // overwrite the transaction id.
        bytes[0] = (transaction_id >> 8) as u8;
        bytes[1] = (transaction_id & 0xFF) as u8;
        bytes.freeze()
    }
}

/// Modify the transaction ID of the given query to a random value to prevent poisoning attacks.
fn generate_tid(query: &[u8]) -> (Bytes, u16) {
    let mut rng = rand::rng();

    let randomized_id = rng.random::<u16>();

    let mut bytes = BytesMut::from(&query[0..]);
    // overwrite the transaction id.
    bytes[0] = (randomized_id >> 8) as u8;
    bytes[1] = (randomized_id & 0xFF) as u8;

    (bytes.freeze(), randomized_id)
}
