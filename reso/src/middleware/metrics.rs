use reso_context::{DnsMiddleware, DnsRequestCtx, DnsResponse, ErrorType};

use crate::{
    global::Global,
    local::Local,
    metrics::event::{ErrorLogEvent, QueryLogEvent},
};

/// Middleware that logs query and error metrics.
pub struct MetricsMiddleware;

impl MetricsMiddleware {
    fn record_query(ctx: &mut DnsRequestCtx<Global, Local>, response: &mut DnsResponse) -> anyhow::Result<()> {
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

    fn record_error(ctx: &DnsRequestCtx<Global, Local>, error_type: &ErrorType, message: &str) {
        let local = ctx.local();

        let ts_ms: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("failed to get the system time")
            .as_millis() as i64;

        let mut qname = None;
        let mut qtype = None;

        if let Ok(msg) = ctx.message() {
            qname = msg.questions().first().map(|q| q.qname.to_string());
            qtype = msg.questions().first().map(|q| q.qtype.to_u16() as i64);
        }

        ctx.global().metrics.error(ErrorLogEvent {
            ts_ms,
            client: ctx.request_address().to_string(),
            transport: ctx.request_type(),
            message: message.to_string(),
            r#type: error_type.clone(),
            dur_ms: local.time_elapsed().as_millis() as u64,
            qname,
            qtype,
        });
    }
}

#[async_trait::async_trait]
impl DnsMiddleware<Global, Local> for MetricsMiddleware {
    async fn on_response(
        &self,
        ctx: &mut DnsRequestCtx<Global, Local>,
        response: &mut DnsResponse,
    ) -> anyhow::Result<()> {
        if let Err(e) = Self::record_query(ctx, response) {
            tracing::warn!("failed to record query metrics: {}", e);
        }
        Ok(())
    }

    async fn on_error(&self, ctx: &mut DnsRequestCtx<Global, Local>, error_type: &ErrorType, message: &str) {
        Self::record_error(ctx, error_type, message);
    }
}
