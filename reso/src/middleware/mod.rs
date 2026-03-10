use reso_dns::{DnsMessage, DnsMessageBuilder, Edns};

pub mod blocklist;
pub mod cache;
pub mod local_records;
pub mod metrics;
pub mod ratelimit;
pub mod reso;

pub fn echo_edns(query: &DnsMessage, mut builder: DnsMessageBuilder) -> DnsMessageBuilder {
    if let Some(edns) = query.edns() {
        let mut response_edns = Edns::default();
        response_edns.set_do_bit(edns.do_bit());
        builder = builder.with_edns(response_edns);
    }
    builder
}
