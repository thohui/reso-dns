use std::{
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use bytes::Bytes;

use crate::{reader::DnsMessageReader, writer::DnsMessageWriter};

/// Represent a DNS message (packet)
#[derive(Debug, Clone, PartialEq)]
pub struct DnsMessage {
    /// Transaction id
    pub id: u16,
    /// Flags
    pub flags: DnsFlags,
    /// Questions in the DNS message
    questions: Vec<DnsQuestion>,
    /// Answers in the DNS message
    answers: Vec<DnsRecord>,
    /// Authority records in the DNS message
    authority_records: Vec<DnsRecord>,
    /// Additional records in the DNS message
    additional_records: Vec<DnsRecord>,
    /// EDNS
    edns: Option<Edns>,
}

impl DnsMessage {
    pub fn new(
        id: u16,
        flags: DnsFlags,
        questions: Vec<DnsQuestion>,
        answers: Vec<DnsRecord>,
        authority_records: Vec<DnsRecord>,
        additional_records: Vec<DnsRecord>,
    ) -> Self {
        Self {
            id,
            flags,
            questions,
            answers,
            authority_records,
            additional_records,
            edns: None,
        }
    }

    pub fn decode(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = DnsMessageReader::new(data);

        let id = reader.read_u16()?;
        let flags = DnsFlags::from(reader.read_u16()?);

        let number_of_questions = reader.read_u16()?; // QDCOUNT
        let number_of_answers = reader.read_u16()?; // ANCOUNT
        let number_of_authority_records = reader.read_u16()?; // NSCOUNT
        let number_of_additional_records = reader.read_u16()?; // ARCOUNT

        let mut questions = Vec::with_capacity(number_of_questions as usize);

        for _ in 0..number_of_questions {
            let question = DnsQuestion::read(&mut reader)?;
            questions.push(question);
        }

        let mut answers = Vec::with_capacity(number_of_answers as usize);

        for _ in 0..number_of_answers {
            let answer = DnsRecord::read(&mut reader)?;
            answers.push(answer);
        }

        let mut authority_records = Vec::with_capacity(number_of_authority_records as usize);

        for _ in 0..number_of_authority_records {
            authority_records.push(DnsRecord::read(&mut reader)?);
        }

        let mut additional_records = Vec::with_capacity(number_of_additional_records as usize);

        let mut edns: Option<Edns> = None;

        for _ in 0..number_of_additional_records {
            let start = reader.position();
            let _ = reader.read_qname();

            let rtype = RecordType::try_from(reader.read_u16()?)?;

            // Handle EDNS
            if rtype == RecordType::OPT {
                let udp_payload_size = reader.read_u16()?;

                // TTL packed: ext_rcode | version | z_flags
                let ttl = reader.read_u32()?;

                let extended_rcode = ((ttl >> 24) & 0xFF) as u8;
                let version = ((ttl >> 16) & 0xFF) as u8;
                let z_flags = (ttl & 0xFFFF) as u16;

                // RDLEN + options;
                let rdlen = reader.read_u16()? as usize;
                let opts_end = reader.position() + rdlen;

                let mut options = Vec::new();

                while reader.position() < opts_end {
                    let code = reader.read_u16()?;
                    let len = reader.read_u16()? as usize;
                    let data = reader.read_bytes(len)?;
                    match code {
                        8 => {
                            // ECS
                            if data.len() >= 4 {
                                let family = u16::from_be_bytes([data[0], data[1]]);
                                let source_prefix = data[2];
                                let scope_prefix = data[3];
                                let addr = data[4..].to_vec();
                                options.push(EdnsOption::ClientSubnet {
                                    family,
                                    source_prefix,
                                    scope_prefix,
                                    address: addr,
                                });
                            } else {
                                println!("unknown edns option: {}", code);
                            }
                        }
                        10 => {
                            // Cookie
                            options.push(EdnsOption::Cookie(data.to_vec()));
                        }
                        _ => {
                            println!("unknown edns option: {}", code);
                        }
                    }
                }
                edns = Some(Edns {
                    udp_payload_size,
                    extended_rcode,
                    version,
                    z_flags,
                    options,
                });
            } else {
                // Not OPT: handle as normal record.
                reader.seek(start)?;
                additional_records.push(DnsRecord::read(&mut reader)?);
            }
        }

        Ok(Self {
            id,
            flags,
            questions,
            answers,
            authority_records,
            additional_records,
            edns,
        })
    }

    pub fn encode(&self) -> anyhow::Result<Bytes> {
        let mut writer = DnsMessageWriter::new();

        // ID
        writer.write_u16(self.id)?;

        // Flags
        self.flags.write(&mut writer)?;

        // QDCOUNT
        writer.write_u16(self.questions.len() as u16)?;

        // ANCOUNT
        writer.write_u16(self.answers.len() as u16)?;

        // NSCOUNT
        writer.write_u16(self.authority_records.len() as u16)?;

        // ARCOUNT
        writer.write_u16(self.additional_records.len() as u16)?;

        // Questions
        for question in &self.questions {
            question.write(&mut writer)?;
        }

        // Answers
        for answer in &self.answers {
            answer.write(&mut writer)?;
        }

        // Authority records
        for authority_record in &self.authority_records {
            authority_record.write(&mut writer)?;
        }

        // Additional records
        for additional_record in &self.additional_records {
            additional_record.write(&mut writer)?;
        }

        Ok(writer.into_bytes())
    }

    /// Questions
    pub fn questions(&self) -> &[DnsQuestion] {
        &self.questions
    }

    /// Answers
    pub fn answers(&self) -> &[DnsRecord] {
        &self.answers
    }

    /// Authority records
    pub fn authority_records(&self) -> &[DnsRecord] {
        &self.authority_records
    }

    /// Additional records
    pub fn additional_records(&self) -> &[DnsRecord] {
        &self.additional_records
    }

    /// EDNS
    pub fn edns(&self) -> &Option<Edns> {
        &self.edns
    }

    pub fn rcode(&self) -> DnsResponseCode {
        let low = self.flags.rcode_low as u16;
        let high = self.edns.as_ref().map(|e| e.extended_rcode).unwrap_or(0) as u16;
        DnsResponseCode::from((high << 4) | low)
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct DnsFlags {
    /// Query or Response
    pub qr: bool,
    /// Opcode
    pub opcode: DnsOpcode,
    /// Authoritative Answer
    pub aa: bool,
    /// Truncated, indicates that this message was truncated due to length greater than 512 bytes
    pub tc: bool,
    /// Recursion Desired, indicates that the client desires recursive resolution
    pub rd: bool,
    /// Recursion Available, indicates that the server supports recursive resolution
    pub ra: bool,
    /// Z flag, reserved for future use, must be zero in all queries and responses
    pub z: bool,
    /// Authentic Data, indicates that the response is authentic
    pub ad: bool,
    /// Checking Disabled, indicates that the server is not performing DNSSEC validation
    pub cd: bool,
    // Lower part of the response code.
    pub rcode_low: u8,
}

impl From<u16> for DnsFlags {
    fn from(bytes: u16) -> Self {
        Self {
            qr: (bytes >> 15) & 0x1 != 0,
            opcode: DnsOpcode::from(((bytes >> 11) & 0xF) as u8),
            aa: (bytes >> 10) & 0x1 != 0,
            tc: (bytes >> 9) & 0x1 != 0,
            rd: (bytes >> 8) & 0x1 != 0,
            ra: (bytes >> 7) & 0x1 != 0,
            z: (bytes >> 6) & 0x1 != 0,
            ad: (bytes >> 5) & 0x1 != 0,
            cd: (bytes & 0x1) != 0,
            rcode_low: (bytes & 0x0f) as u8,
        }
    }
}

impl DnsFlags {
    pub fn write(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        let opcode: u8 = self.opcode.into();
        writer.write_u16(
            ((self.qr as u16) << 15)
                | ((opcode as u16) << 11)
                | ((self.aa as u16) << 10)
                | ((self.tc as u16) << 9)
                | ((self.rd as u16) << 8)
                | ((self.ra as u16) << 7)
                | ((self.z as u16) << 6)
                | ((self.ad as u16) << 5)
                | (self.cd as u16) << 4
                | self.rcode_low as u16, // todo: add edns support for this. should probably move this inside the encode fn.
        )?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[repr(u16)]
pub enum DnsResponseCode {
    /// No error, the request was successful
    #[default]
    NoError = 0,
    /// Format error, the request was malformed
    FormatError = 1,
    /// Server failure, the server encountered an error while processing the request
    ServerFailure = 2,
    /// Non-existent domain, the requested domain does not exist
    NxDomain = 3,
    /// Unknown
    Unknown(u16),
}

impl From<DnsResponseCode> for u8 {
    fn from(value: DnsResponseCode) -> Self {
        match value {
            DnsResponseCode::NoError => 0,
            DnsResponseCode::FormatError => 1,
            DnsResponseCode::ServerFailure => 2,
            DnsResponseCode::NxDomain => 3,
            DnsResponseCode::Unknown(v) => (v & 0x0F) as u8, // only lower 4 bits
        }
    }
}

impl From<u16> for DnsResponseCode {
    fn from(value: u16) -> Self {
        match value {
            0 => DnsResponseCode::NoError,
            1 => DnsResponseCode::FormatError,
            2 => DnsResponseCode::ServerFailure,
            3 => DnsResponseCode::NxDomain,
            v => DnsResponseCode::Unknown(v),
        }
    }
}

impl From<DnsResponseCode> for u16 {
    fn from(value: DnsResponseCode) -> u16 {
        match value {
            DnsResponseCode::NoError => 0,
            DnsResponseCode::FormatError => 1,
            DnsResponseCode::ServerFailure => 2,
            DnsResponseCode::NxDomain => 3,
            DnsResponseCode::Unknown(v) => v,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[repr(u8)]
pub enum DnsOpcode {
    /// Standard query
    #[default]
    Query = 0,
    /// Inverse query, obsolete
    IQuery = 1,
    /// Server status request, obsolete
    Status = 2,
    /// Unknown
    Unknown(u8),
}

impl From<u8> for DnsOpcode {
    fn from(value: u8) -> Self {
        match value {
            0 => DnsOpcode::Query,
            1 => DnsOpcode::IQuery,
            2 => DnsOpcode::Status,
            v => DnsOpcode::Unknown(v),
        }
    }
}

impl From<DnsOpcode> for u8 {
    fn from(value: DnsOpcode) -> Self {
        match value {
            DnsOpcode::Query => 0,
            DnsOpcode::IQuery => 1,
            DnsOpcode::Status => 2,
            DnsOpcode::Unknown(v) => v,
        }
    }
}

/// Represents a DNS question in a DNS message.
#[derive(Debug, Clone, PartialEq)]
pub struct DnsQuestion {
    /// The domain name being queried
    pub qname: Arc<str>,
    /// The type of the query (e.g., A, AAAA, CNAME)
    pub qtype: RecordType,
    /// The class of the query (e.g., IN for Internet)
    pub qclass: ClassType,
}

impl DnsQuestion {
    pub fn new(qname: Arc<str>, qtype: RecordType, qclass: ClassType) -> Self {
        Self {
            qname,
            qtype,
            qclass,
        }
    }

    /// Create a new DNS question from a reader.
    pub fn read(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let qname: Arc<str> = reader.read_qname()?.into();
        let qtype = RecordType::try_from(reader.read_u16()?)?;
        let qclass = ClassType::from(reader.read_u16()?);

        Ok(Self {
            qname,
            qtype,
            qclass,
        })
    }

    /// Write the DNS question to the writer.
    pub fn write(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        writer.write_qname(&self.qname)?;
        writer.write_u16(self.qtype as u16)?;
        writer.write_u16(self.qclass as u16)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u16)]
pub enum RecordType {
    /// IPv4
    A = 1,
    /// Name server
    NS = 2,
    /// Canonical name
    CNAME = 5,
    /// Start of authority
    SOA = 6,
    /// Pointer (for reverse DNS)
    PTR = 12,
    /// Mail exchange
    MX = 15,
    /// Text record
    TXT = 16,
    /// IPv6
    AAAA = 28,
    /// Service locator
    SRV = 33,
    /// OPT, only used by additional records (EDNS)
    OPT = 41,
    /// All records
    ANY = 255,
}

impl TryFrom<u16> for RecordType {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RecordType::A),
            2 => Ok(RecordType::NS),
            5 => Ok(RecordType::CNAME),
            6 => Ok(RecordType::SOA),
            12 => Ok(RecordType::PTR),
            15 => Ok(RecordType::MX),
            16 => Ok(RecordType::TXT),
            28 => Ok(RecordType::AAAA),
            33 => Ok(RecordType::SRV),
            41 => Ok(RecordType::OPT),
            255 => Ok(RecordType::ANY),
            _ => Err(anyhow::format_err!("unknown record type {}", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u16)]
pub enum ClassType {
    /// Internet
    IN = 1,
    /// Chaosnet
    CH = 3,
    /// Hesoid (MIT Athena)
    HS = 4,
    /// Any
    Any = 255,
}

impl From<u16> for ClassType {
    // todo: refactor this to tryfrom.
    fn from(value: u16) -> Self {
        match value {
            1 => ClassType::IN,
            3 => ClassType::CH,
            4 => ClassType::HS,
            255 => ClassType::Any,
            _ => panic!("Unknown class type: {}", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    Raw(Vec<u8>),
    Ipv4(std::net::Ipv4Addr),
    Ipv6(std::net::Ipv6Addr),
    Text(Arc<str>),
    Soa {
        /// Primary nameserver.
        mname: Arc<str>,
        /// Contact email
        rname: Arc<str>,
        /// Serial
        serial: u32,
        /// Refresh
        refresh: u32,
        /// Retry
        retry: u32,
        /// Expire
        expire: u32,
        /// Minimum
        minimum: u32,
    },
    DomainName(Arc<str>),
}

impl DnsRecordData {
    pub fn write(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        match self {
            DnsRecordData::Raw(data) => writer.write_bytes(data),
            DnsRecordData::Ipv4(addr) => writer.write_bytes(&addr.octets()),
            DnsRecordData::Ipv6(addr) => writer.write_bytes(&addr.octets()),
            DnsRecordData::Text(text) => writer.write_bytes(text.as_bytes()),
            DnsRecordData::DomainName(name) => writer.write_qname(name),

            DnsRecordData::Soa {
                mname,
                rname,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            } => unimplemented!(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            DnsRecordData::Raw(data) => data.len(),
            DnsRecordData::Ipv4(_) => 4,  // 4 bytes for IPv4 address
            DnsRecordData::Ipv6(_) => 16, // 16 bytes for IPv6 address
            DnsRecordData::Text(text) => text.len() + 1, // +1 for the length byte
            DnsRecordData::DomainName(name) => name.len() + 2, // +2 for the length byte and null terminator
            DnsRecordData::Soa {
                mname,
                rname,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            } => unimplemented!(),
        }
    }

    pub fn to_str(&self) -> Option<String> {
        match self {
            DnsRecordData::Raw(_) => None,
            DnsRecordData::Ipv4(addr) => Some(addr.to_string()),
            DnsRecordData::Ipv6(addr) => Some(addr.to_string()),
            DnsRecordData::Text(text) => Some(text.to_string()),
            DnsRecordData::DomainName(name) => Some(name.to_string()),
            DnsRecordData::Soa {
                mname,
                rname,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    pub name: Arc<str>,
    pub record_type: RecordType,
    pub class: ClassType,
    pub ttl: u32,
    pub data: DnsRecordData,
}

impl DnsRecord {
    /// Create a new DNS record.
    pub fn read(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let name = reader.read_qname()?.into();
        let r#type = RecordType::try_from(reader.read_u16()?)?;
        let class = ClassType::from(reader.read_u16()?);
        let ttl = reader.read_u32()?;
        let data_length = reader.read_u16()? as usize;

        let data = match r#type {
            RecordType::CNAME | RecordType::PTR | RecordType::NS => {
                let domain_name = reader.read_qname()?;
                DnsRecordData::DomainName(domain_name.into())
            }
            RecordType::A => {
                let raw_data = reader.read_bytes(4)?;
                let ipv4_addr = Ipv4Addr::new(raw_data[0], raw_data[1], raw_data[2], raw_data[3]);
                DnsRecordData::Ipv4(ipv4_addr)
            }
            RecordType::AAAA => {
                let raw_data = reader.read_bytes(16)?;
                let ipv6_addr = Ipv6Addr::new(
                    u16::from_be_bytes([raw_data[0], raw_data[1]]),
                    u16::from_be_bytes([raw_data[2], raw_data[3]]),
                    u16::from_be_bytes([raw_data[4], raw_data[5]]),
                    u16::from_be_bytes([raw_data[6], raw_data[7]]),
                    u16::from_be_bytes([raw_data[8], raw_data[9]]),
                    u16::from_be_bytes([raw_data[10], raw_data[11]]),
                    u16::from_be_bytes([raw_data[12], raw_data[13]]),
                    u16::from_be_bytes([raw_data[14], raw_data[15]]),
                );
                DnsRecordData::Ipv6(ipv6_addr)
            }
            RecordType::TXT => {
                let text_length = reader.read_u8()? as usize;
                let text = reader.read_bytes(text_length)?;
                let utf_str = String::from_utf8(text.to_vec())?;
                DnsRecordData::Text(utf_str.into())
            }
            _ => {
                let raw_data = reader.read_bytes(data_length)?;
                DnsRecordData::Raw(raw_data.into())
            }
        };

        Ok(Self {
            name,
            record_type: r#type,
            class,
            ttl,
            data,
        })
    }

    /// Write the DNS record to the DNS message.
    pub fn write(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        writer.write_qname(&self.name)?;
        writer.write_u16(self.record_type as u16)?;
        writer.write_u16(self.class as u16)?;
        writer.write_u32(self.ttl)?;
        writer.write_u16(self.data.len() as u16)?;
        self.data.write(writer)?;
        Ok(())
    }

    /// Get the name of the DNS record.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Get the type of the DNS record.
    pub fn record_type(&self) -> RecordType {
        self.record_type
    }
    /// Get the class of the DNS record.
    pub fn class(&self) -> ClassType {
        self.class
    }
    /// Get the TTL (Time to Live) of the DNS record.
    pub fn ttl(&self) -> u32 {
        self.ttl
    }
    /// Get the data of the DNS record.
    pub fn data(&self) -> &DnsRecordData {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edns {
    /// From OPT.CLASS (not a DNS class): max UDP payload size sender can handle
    pub udp_payload_size: u16,
    /// High bits of RCODE (ttl[31:24])
    pub extended_rcode: u8,
    /// EDNS version (ttl[23:16]) — must be 0 today
    pub version: u8,
    /// Z flags (ttl[15:0]); DO (DNSSEC OK) is 0x8000
    pub z_flags: u16,
    /// Edns option
    pub options: Vec<EdnsOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdnsOption {
    /// RFC 7873
    Cookie(Vec<u8>),
    /// RFC 7871
    ClientSubnet {
        family: u16,
        source_prefix: u8,
        scope_prefix: u8,
        address: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {

    use crate::builder::DnsMessageBuilder;

    use super::*;

    #[test]

    fn test_writer_reader() {
        let mut writer = DnsMessageWriter::new();
        writer.write_u8(42).unwrap();
        writer.write_u16(12345).unwrap();
        writer.write_u32(67890).unwrap();
        writer.write_qname("example.com").unwrap();
        let bytes = writer.into_bytes();

        let mut reader = DnsMessageReader::new(&bytes);
        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.read_u16().unwrap(), 12345);
        assert_eq!(reader.read_u32().unwrap(), 67890);
        assert_eq!(reader.read_qname().unwrap(), "example.com");
    }

    #[test]
    fn test_encode_decode() {
        let packet = DnsMessageBuilder::new()
            .with_id(12345)
            .with_flags(DnsFlags {
                qr: true,
                opcode: DnsOpcode::Query,
                aa: false,
                tc: false,
                rd: true,
                ra: false,
                z: false,
                ad: false,
                cd: false,
                rcode_low: 0,
            })
            .add_question(DnsQuestion {
                qname: Arc::<str>::from("example.com"),
                qtype: RecordType::A,
                qclass: ClassType::IN,
            })
            .build();

        let bytes = packet.encode().unwrap();
        let decoded_message = DnsMessage::decode(&bytes).unwrap();

        assert_eq!(decoded_message.id, 12345);
        assert_eq!(decoded_message.questions.len(), 1);
        assert_eq!(&*decoded_message.questions[0].qname, "example.com");
        assert_eq!(decoded_message.questions[0].qtype, RecordType::A);
        assert_eq!(decoded_message.questions[0].qclass, ClassType::IN);
        assert!(decoded_message.flags.qr);
        assert_eq!(decoded_message.flags.opcode, DnsOpcode::Query);
    }
    #[test]
    fn test_ns_query_encoding() {
        let packet = DnsMessageBuilder::new()
            .add_question(DnsQuestion::new(
                "com.".into(),
                RecordType::NS,
                ClassType::IN,
            ))
            .build();

        let encoded = packet.encode().unwrap();
        let hex: Vec<String> = encoded.iter().map(|b| format!("{:02x}", b)).collect();
        println!("Packet: {}", hex.join(" "));

        assert!(hex.contains(&"00".to_string()));
        assert!(hex.contains(&"02".to_string())); // QTYPE = 2
        assert!(hex.contains(&"00".to_string()));
        assert!(hex.contains(&"01".to_string())); // QCLASS = 1
    }
}
