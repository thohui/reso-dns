pub mod builder;
pub mod domain_name;
pub mod helpers;
#[macro_use]
mod macros;
pub mod error;
pub mod message;
pub mod reader;
pub mod writer;

pub use error::{DnsError, DnsReadError, DnsWriteError, Result};

pub use builder::DnsMessageBuilder;
pub use message::{
    ClassType, DnsFlags, DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode, Edns, EdnsOption, RecordType,
};

pub use reader::DnsMessageReader;
pub use writer::DnsMessageWriter;
