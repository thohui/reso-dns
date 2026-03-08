use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use rand::Rng;
use reso_context::DnsRequestCtx;
use reso_dns::{
    ClassType, DnsMessage, RecordType,
    domain_name::DomainName,
    message::{ClientSubnet, EdnsOptionData},
};
use reso_inflight::Inflight;

use crate::{DnsResolver, DnsResponse, ResolveError};

use super::{
    request::UpstreamResolveRequest,
    upstream::{Limits, Upstreams},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InflightCacheKey {
    pub name: DomainName,
    pub record_type: RecordType,
    pub class_type: ClassType,
    pub do_bit: bool,
    pub client_subnet: Option<ClientSubnet>,
}

impl TryFrom<&DnsMessage> for InflightCacheKey {
    type Error = anyhow::Error;
    fn try_from(message: &DnsMessage) -> Result<Self, Self::Error> {
        let client_subnet: Option<ClientSubnet> = message.edns().as_ref().and_then(|e| {
            e.options.iter().find_map(|opt| match &opt.data {
                Some(EdnsOptionData::ClientSubnet(cs)) => Some(cs.clone()),
                _ => None,
            })
        });

        message
            .questions()
            .first()
            .map(|q| InflightCacheKey {
                name: q.qname.clone(),
                class_type: q.qclass,
                record_type: q.qtype,
                client_subnet: client_subnet,
                do_bit: message.edns().as_ref().map(|e| e.do_bit()).unwrap_or(false),
            })
            .ok_or_else(|| anyhow::anyhow!("no question in message"))
    }
}

/// Resolver that forwards the incoming request to a defined upstream server.
pub struct ForwardResolver {
    upstreams: Arc<Upstreams>,
    inflight_requests: Inflight<InflightCacheKey, DnsResponseBytes>,
}

impl ForwardResolver {
    pub async fn new(upstreams: &[SocketAddr]) -> anyhow::Result<Self> {
        if upstreams.is_empty() {
            tracing::warn!("No upstreams configured for forward resolver, it will not be able to resolve any queries!");
        }

        tracing::debug!("creating new ForwardResolver instance with upstreams: {:?}", upstreams);

        Ok(Self {
            upstreams: Arc::new(
                Upstreams::new(
                    upstreams,
                    // TODO: make this configurable by the client.
                    Limits {
                        connect_timeout: Duration::from_secs(2),
                        max_tcp_connections: 10,
                        max_idle_tcp_connections: 5,
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
    async fn resolve(&self, ctx: &DnsRequestCtx<G, L>) -> Result<DnsResponse, ResolveError> {
        let query_message = ctx.message().map_err(|e| ResolveError::InvalidRequest(e.to_string()))?;

        if query_message.questions().len() != 1 {
            return Err(ResolveError::InvalidRequest(format!(
                "request contains {} questions, expected 1",
                query_message.questions().len(),
            )));
        }

        let key = InflightCacheKey::try_from(query_message).map_err(|e| ResolveError::Other(e.to_string()))?;

        let upstreams = self.upstreams.clone();

        let query = ctx.raw();
        let request_type = ctx.request_type();
        let budget = *ctx.budget();

        let resp_arc = self
            .inflight_requests
            .get_or_run(key, async move |_| {
                let (randomized_query, _) = generate_tid(&query);

                let request = UpstreamResolveRequest::new(request_type, randomized_query, budget, upstreams);

                let response = request.resolve().await?;

                Ok(DnsResponseBytes::new(response))
            })
            .await
            .map_err(|e| match e.downcast::<ResolveError>() {
                Ok(e) => e,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("inflight cancelled") {
                        ResolveError::Timeout
                    } else {
                        ResolveError::Other(msg)
                    }
                }
            })?;

        let response = resp_arc.as_ref().clone().into_custom_response(query_message.id);

        let response_message =
            DnsMessage::decode(&response).map_err(|e| ResolveError::InvalidResponse(e.to_string()))?;

        validate_upstream_response(&query_message, &response_message)?;

        Ok(DnsResponse::from_parsed(response, response_message))
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

pub fn validate_upstream_response(request: &DnsMessage, response: &DnsMessage) -> Result<(), ResolveError> {
    if request.id != response.id {
        return Err(ResolveError::MalformedResponse("transaction id match".into()));
    }

    if !response.flags.response {
        return Err(ResolveError::MalformedResponse(
            "received query instead of response from upstream".into(),
        ));
    }

    if response.flags.opcode != request.flags.opcode {
        return Err(ResolveError::MalformedResponse("opcode mismatch".into()));
    }

    if request.questions() != response.questions() {
        return Err(ResolveError::MalformedResponse("questions mismatch".into()));
    }

    Ok(())
}
