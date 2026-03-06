use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse};

use crate::{global::Global, local::Local, metrics::event::QueryLogEvent};

/// Middleware that logs query metrics and optionally caches responses.
pub struct MetricsMiddleware;

#[async_trait::async_trait]
impl DnsMiddleware<Global, Local> for MetricsMiddleware {
    async fn on_response(&self, ctx: &DnsRequestCtx<Global, Local>, response: &mut DnsResponse) -> anyhow::Result<()> {
        let message = ctx.message()?;

        let local = ctx.local();

        let ts_ms: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let question = message
            .questions()
            .first()
            .ok_or_else(|| anyhow::anyhow!("no question in message"))?;

        ctx.global().metrics.query(QueryLogEvent {
            ts_ms,
            transport: ctx.request_type(),
            client: ctx.request_address().to_string(),
            qname: question.qname.clone(),
            qtype: question.qtype,
            rcode: response.message()?.response_code()?,
            dur_ms: local.time_elapsed().as_millis() as u64,
            cache_hit: local.cache_hit,
            blocked: local.blocked,
            rate_limited: local.rate_limited,
        });

        Ok(())
    }
}
