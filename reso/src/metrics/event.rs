use reso_context::RequestType;
use reso_dns::{DnsResponseCode, domain_name::DomainName, message::RecordType};

use crate::database::models::query_log::DnsQueryLog;

pub type TsMs = i64;

#[derive(Debug, Clone)]
pub struct QueryLogEvent {
    /// Timestamp in milliseconds.
    pub ts_ms: TsMs,
    /// Transport
    pub transport: RequestType,
    /// Client IP
    pub client: String,
    /// Domain Name
    pub qname: DomainName,
    /// Record Type
    pub qtype: RecordType,
    /// Response code
    pub rcode: DnsResponseCode,
    /// Duration in microseconds.
    pub dur_us: u32,
    /// Cache hit
    pub cache_hit: bool,
    /// Blocked
    pub blocked: bool,
}

impl QueryLogEvent {
    pub fn into_db_model(self) -> DnsQueryLog {
        DnsQueryLog {
            ts_ms: self.ts_ms,
            blocked: self.blocked,
            transport: self.transport as u8 as i64,
            client: self.client,
            cache_hit: self.cache_hit,
            dur_us: self.dur_us as i64,
            qname: self.qname.to_string(),
            qtype: self.qtype as u16 as i64,
            rcode: self.rcode as u16 as i64,
        }
    }
}
