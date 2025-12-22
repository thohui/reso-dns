use std::{
    hash::Hash,
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use bytes::Bytes;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{
    domain_name::DomainName,
    reader::{DnsMessageReader, DnsReadable},
    writer::{DnsMessageWriter, DnsWritable},
};

/// Represents a DNS message.
/// This struct encapsulates the various components of a DNS messag and doesn not represent the full wire structure.
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
        let flags = DnsFlags::read_from(&mut reader)?;

        let number_of_questions = reader.read_u16()?; // QDCOUNT
        let number_of_answers = reader.read_u16()?; // ANCOUNT
        let number_of_authority_records = reader.read_u16()?; // NSCOUNT
        let number_of_additional_records = reader.read_u16()?; // ARCOUNT

        let mut questions = Vec::with_capacity(number_of_questions as usize);

        for _ in 0..number_of_questions {
            let question = DnsQuestion::read_from(&mut reader)?;
            questions.push(question);
        }

        let mut answers = Vec::with_capacity(number_of_answers as usize);

        for _ in 0..number_of_answers {
            let answer = DnsRecord::read_from(&mut reader)?;
            answers.push(answer);
        }

        let mut authority_records = Vec::with_capacity(number_of_authority_records as usize);

        for _ in 0..number_of_authority_records {
            authority_records.push(DnsRecord::read_from(&mut reader)?);
        }

        let mut additional_records = Vec::with_capacity(number_of_additional_records as usize);

        let mut edns: Option<Edns> = None;

        for _ in 0..number_of_additional_records {
            let start = reader.position();
            let _ = reader.read_qname();

            let rtype = RecordType::try_from(reader.read_u16()?)?;

            // Handle EDNS
            if rtype == RecordType::OPT {
                edns = Some(Edns::read_from(&mut reader)?)
            } else {
                // Not OPT: handle as normal record.
                reader.seek(start)?;
                additional_records.push(DnsRecord::read_from(&mut reader)?);
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
        self.flags.write_to(&mut writer)?;

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
            question.write_to(&mut writer)?;
        }

        // Answers
        for answer in &self.answers {
            answer.write_to(&mut writer)?;
        }

        // Authority records
        for authority_record in &self.authority_records {
            authority_record.write_to(&mut writer)?;
        }

        // Additional records
        for additional_record in &self.additional_records {
            additional_record.write_to(&mut writer)?;
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

    // Set the response code
    pub fn set_response_code(&mut self, response_code: DnsResponseCode) {
        let full: u16 = response_code.into();
        self.flags.rcode_low = (full & 0x0F) as u8;

        // handle higher part.
        if full > 0x0F {
            let edns = self.edns.get_or_insert_with(Edns::default);
            edns.extended_rcode = (full >> 4) as u8;
        }
    }

    /// Response code
    pub fn response_code(&self) -> anyhow::Result<DnsResponseCode> {
        let low = self.flags.rcode_low as u16;
        let high = self.edns.as_ref().map(|e| e.extended_rcode).unwrap_or(0) as u16;
        let code = DnsResponseCode::try_from((high << 4) | low)?;
        Ok(code)
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct DnsFlags {
    /// Query or Response
    pub response: bool,
    /// Opcode
    pub opcode: DnsOpcode,
    /// Authoritative Answer
    pub authorative_answer: bool,
    /// Truncated, indicates that this message was truncated due to length greater than 512 bytes
    pub truncated: bool,
    /// Recursion Desired, indicates that the client desires recursive resolution
    pub recursion_desired: bool,
    /// Recursion Available, indicates that the server supports recursive resolution
    pub recursion_available: bool,
    /// Z flag, reserved for future use, must be zero in all queries and responses
    pub(crate) z: bool,
    /// Authentic Data, indicates that the response is authentic
    pub authentic_data: bool,
    /// Checking Disabled, indicates that the server is not performing DNSSEC validation
    pub checking_disabled: bool,
    // Lower part of the response code.
    pub(crate) rcode_low: u8,
}

impl TryFrom<u16> for DnsFlags {
    type Error = anyhow::Error;

    fn try_from(bytes: u16) -> Result<Self, Self::Error> {
        Ok(Self {
            response: (bytes >> 15) & 0x1 != 0,
            opcode: DnsOpcode::try_from(((bytes >> 11) & 0xF) as u8)?,
            authorative_answer: (bytes >> 10) & 0x1 != 0,
            truncated: (bytes >> 9) & 0x1 != 0,
            recursion_desired: (bytes >> 8) & 0x1 != 0,
            recursion_available: (bytes >> 7) & 0x1 != 0,
            z: (bytes >> 6) & 0x1 != 0,
            authentic_data: (bytes >> 5) & 0x1 != 0,
            checking_disabled: (bytes >> 4) & 0x1 != 0,
            rcode_low: (bytes & 0x0F) as u8,
        })
    }
}

impl DnsFlags {
    pub fn new(
        response: bool,
        opcode: DnsOpcode,
        authorative_answer: bool,
        truncated: bool,
        recursion_desired: bool,
        recursion_available: bool,
        authentic_data: bool,
        checking_disabled: bool,
    ) -> Self {
        Self {
            response,
            opcode,
            authorative_answer,
            truncated,
            recursion_desired,
            recursion_available,
            z: false,
            authentic_data,
            checking_disabled,
            rcode_low: 0,
        }
    }
}

impl DnsReadable for DnsFlags {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let bytes = reader.read_u16()?;
        Ok(Self {
            response: (bytes >> 15) & 0x1 != 0,
            opcode: DnsOpcode::try_from(((bytes >> 11) & 0xF) as u8)?,
            authorative_answer: (bytes >> 10) & 0x1 != 0,
            truncated: (bytes >> 9) & 0x1 != 0,
            recursion_desired: (bytes >> 8) & 0x1 != 0,
            recursion_available: (bytes >> 7) & 0x1 != 0,
            z: (bytes >> 6) & 0x1 != 0,
            authentic_data: (bytes >> 5) & 0x1 != 0,
            checking_disabled: (bytes >> 4) & 0x1 != 0,
            rcode_low: (bytes & 0x0F) as u8,
        })
    }
}

impl DnsWritable for DnsFlags {
    fn write_to(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        let opcode: u8 = self.opcode.into();
        writer.write_u16(
            ((self.response as u16) << 15)
                | ((opcode as u16) << 11)
                | ((self.authorative_answer as u16) << 10)
                | ((self.truncated as u16) << 9)
                | ((self.recursion_desired as u16) << 8)
                | ((self.recursion_available as u16) << 7)
                | ((self.z as u16) << 6)
                | ((self.authentic_data as u16) << 5)
                | (self.checking_disabled as u16) << 4
                | self.rcode_low as u16, // todo: add edns support for this. should probably move this inside the encode fn.
        )?;
        Ok(())
    }
}

/// Dns response code
///
/// Based on: https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-6
#[derive(Debug, Copy, Clone, Default, PartialEq, TryFromPrimitive, IntoPrimitive)]
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
    /// Not Implemented
    NotImp = 4,
    /// Query refused
    Refused = 5,
    /// Name Exists when it should not
    YXDomain = 6,
    /// RR Set Exists when it should not
    YXRRSet = 7,
    /// RR Set that should exist does not
    NXRRSet = 8,
    /// Server Not Authoritative for zone
    ///
    /// Not Authorized
    NotAuth = 9,
    /// Name not contained in zone
    NotZone = 10,
    /// DSO-TYPE Not Implemented
    DSOTYPENI = 11,
    /// Bad OPT Version
    /// TSIG Signature Failure
    BADVERS = 16,
    /// Key not recognized
    BADKEY = 17,
    /// Signature out of time window
    BADTIME = 18,
    /// Bad TKEY Mode
    BADMODE = 19,
    /// Duplicate key name
    BADNAME = 20,
    /// Algorithm not supported
    BADALG = 21,
    /// Bad Truncation
    BADTRUNC = 22,
    /// Bad/missing Server Cookie
    BADCOOKIE = 23,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum DnsOpcode {
    /// Standard query
    #[default]
    Query = 0,
    /// Inverse query, obsolete
    IQuery = 1,
    /// Server status request, obsolete
    Status = 2,
}

/// Represents a DNS question in a DNS message.
#[derive(Debug, Clone, PartialEq)]
pub struct DnsQuestion {
    /// The domain name being queried
    pub qname: DomainName,
    /// The type of the query (e.g., A, AAAA, CNAME)
    pub qtype: RecordType,
    /// The class of the query (e.g., IN for Internet)
    pub qclass: ClassType,
}

impl DnsQuestion {
    pub fn new(qname: DomainName, qtype: RecordType, qclass: ClassType) -> Self {
        Self { qname, qtype, qclass }
    }
}

impl DnsReadable for DnsQuestion {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let qname = reader.read_qname()?;
        let qtype = RecordType::try_from(reader.read_u16()?)?;
        let qclass = ClassType::try_from(reader.read_u16()?)?;

        Ok(Self { qname, qtype, qclass })
    }
}

impl DnsWritable for DnsQuestion {
    fn write_to(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        writer.write_qname(&self.qname)?;
        writer.write_u16(self.qtype as u16)?;
        writer.write_u16(self.qclass as u16)?;
        Ok(())
    }
}

/// DNS record types.
///
/// Based on: https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-4
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum RecordType {
    /// IPv4
    A = 1,
    /// Name server
    NS = 2,
    /// Mail destination
    MD = 3,
    /// Mail forwarder
    MF = 4,
    /// Canonical name
    CNAME = 5,
    /// Start of authority
    SOA = 6,
    /// MB
    MB = 7,
    /// MG
    MG = 8,
    /// MR
    MR = 9,
    /// Null
    NULL = 10,
    /// Well known service description
    WKS = 11,
    /// Pointer (for reverse DNS)
    PTR = 12,
    /// HINFO
    HINFO = 13,
    /// MINFO
    MINFO = 14,
    /// Mail exchange
    MX = 15,
    /// Text strings
    TXT = 16,
    /// for Responsible Person
    RP = 17,
    /// for AFS Data Base location
    AFSDB = 18,
    /// for X.25 PSDN address
    X25 = 19,
    /// for ISDN address
    ISDN = 20,
    /// for Route Through
    RT = 21,
    /// for NSAP address, NSAP style A record (DEPRECATED)
    NSAP = 22,
    /// for domain name pointer, NSAP style (DEPRECATED)
    NSAPPTR = 23,
    /// for security signature
    SIG = 24,
    /// for security key
    KEY = 25,
    /// X.400 mail mapping information
    PX = 26,
    /// Geographical Position
    GPOS = 27,
    /// IPv6
    AAAA = 28,
    /// Location Information
    LOC = 29,
    /// Next Domain (OBSOLETE)
    NXT = 30,
    /// Endpoint Identifier
    EID = 31,
    /// Nimrod Locator
    NIMLOC = 32,
    /// Service locator
    SRV = 33,
    /// ATM Address
    ATMA = 34,
    /// Naming Authority Pointer
    NAPTR = 35,
    /// Key Exchanger
    KX = 36,
    /// CERT
    CERT = 37,
    /// A6 (OBSOLETE - use AAAA)
    A6 = 38,
    /// DNAME
    DNAME = 39,
    /// SINK
    SINK = 40,
    /// OPT, only used by additional records (EDNS)
    OPT = 41,
    /// APL
    APL = 42,
    /// Delegation Signer
    DS = 43,
    /// SSH Key Fingerprint
    SSHFP = 44,
    /// IP SEC KEY
    IPSECKEY = 45,
    /// RRSIG
    RRSIG = 46,
    /// NSEC
    NSEC = 47,
    /// DNS KEY
    DNSKEY = 48,
    /// DHCID
    DHCID = 49,
    /// NSEC3
    NSEC3 = 50,
    /// NSEC3PARAM
    NSEC3PARAM = 51,
    /// TLSA
    TLSA = 52,
    /// S/MIME cert association
    SMIMEA = 53,
    /// Host Identity Protocol
    HIP = 55,
    /// NINFO
    NINFO = 56,
    /// RKEY
    RKEY = 57,
    /// Trust Anchor LINK
    TALINK = 58,
    /// Child DS
    CDS = 59,
    /// DNSKEY(s) the Child wants reflected in DS
    CDNSKEY = 60,
    /// OpenPGP Key
    OPENPGPKEY = 61,
    /// Child-To-Parent Synchronization
    CSYNC = 62,
    /// Message Digest Over Zone Data
    ZONEMD = 63,
    /// General-purpose service binding
    SVCB = 64,
    /// SVCB-compatible type for use with HTTP
    HTTPS = 65,
    /// Endpoint discovery for delegation synchronization
    DSYNC = 66,
    /// Hierarchical Host Identity Tag
    HHIT = 67,
    /// UAS Broadcast Remote Identification
    BRID = 68,
    /// SPF
    SPF = 99,
    /// UINFO
    UINFO = 100,
    /// UID
    UID = 101,
    /// GID
    GID = 102,
    /// UNSPEC
    UNSPEC = 103,
    /// NID
    NID = 104,
    /// L32
    L32 = 105,
    /// L64
    L64 = 106,
    /// LP
    LP = 107,
    /// an EUI-48 address
    EUI48 = 108,
    /// an EUI-64 address
    EUI64 = 109,
    /// NXDOMAIN indicator for Compact Denial of Existence
    NXNAME = 128,
    /// Transaction Key
    TKEY = 249,
    /// Transaction Signature
    TSIG = 250,
    /// Incremental transfer
    IXFR = 251,
    /// transfer of an entire zone
    AXFR = 252,
    /// Mailbox-related RRs (MB, MG or MR)
    MAILB = 253,
    /// Mail agent RRs (OBSOLETE - see MX)
    MAILA = 254,
    /// All records
    ANY = 255,
    /// URI
    URI = 256,
    /// Certification Authority Restriction
    CAA = 257,
    /// Application Visibility and Control
    AVC = 258,
    /// Digital Object Architecture
    DOA = 259,
    /// Automatic Multicast Tunneling Relay
    AMTRELAY = 260,
    // Resolver Information as Key/Value Pairs
    RESINFO = 261,
    /// Public wallet address
    WALLET = 262,
    /// BP Convergence Layer Adapter
    CLA = 263,
    /// BP Node Number
    IPN = 264,
}

/// DNS class types.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, TryFromPrimitive)]
#[repr(u16)]
pub enum ClassType {
    /// Internet
    IN = 1,
    /// Chaosnet
    CH = 3,
    /// Hesoid (MIT Athena)
    HS = 4,
    /// Any
    ANY = 255,
}

/// Associated data for a DNS record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    Raw(Vec<u8>),
    Ipv4(std::net::Ipv4Addr),
    Ipv6(std::net::Ipv6Addr),
    Text(Arc<str>),

    SOA {
        /// Primary nameserver.
        mname: DomainName,
        /// Contact email
        rname: DomainName,
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
    MX {
        priority: u16,
        host: DomainName,
    },
    SRV {
        priority: u16,
        weight: u16,
        port: u16,
        target: DomainName,
    },
    DomainName(DomainName),
}

impl DnsRecordData {
    /// Write the DNS record data to the DNS message.
    pub fn write(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        match self {
            DnsRecordData::Raw(data) => writer.write_bytes(data),
            DnsRecordData::Ipv4(addr) => writer.write_bytes(&addr.octets()),
            DnsRecordData::Ipv6(addr) => writer.write_bytes(&addr.octets()),
            DnsRecordData::Text(text) => writer.write_bytes(text.as_bytes()),
            DnsRecordData::DomainName(name) => writer.write_qname(name),

            DnsRecordData::SOA {
                mname,
                rname,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            } => {
                writer.write_qname(mname)?;
                writer.write_qname(rname)?;
                writer.write_u32(*serial)?;
                writer.write_u32(*refresh)?;
                writer.write_u32(*retry)?;
                writer.write_u32(*expire)?;
                writer.write_u32(*minimum)?;
                Ok(())
            }
            DnsRecordData::MX { priority, host } => {
                writer.write_u16(*priority)?;
                writer.write_qname(host)?;
                Ok(())
            }
            DnsRecordData::SRV {
                priority,
                weight,
                port,
                target,
            } => {
                writer.write_u16(*priority)?;
                writer.write_u16(*weight)?;
                writer.write_u16(*port)?;
                writer.write_qname(target)?;
                Ok(())
            }
        }
    }

    /// Decode record data based on the provided `record_type`.
    pub fn read_from_record_type(
        reader: &mut DnsMessageReader,
        record_type: &RecordType,
        data_length: usize,
    ) -> anyhow::Result<DnsRecordData> {
        Ok(match *record_type {
            RecordType::CNAME | RecordType::PTR | RecordType::NS => {
                let domain_name = reader.read_qname()?;
                DnsRecordData::DomainName(domain_name)
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
            RecordType::TXT | RecordType::SPF => {
                let text_length = reader.read_u8()? as usize;
                let text = reader.read_bytes(text_length)?;
                let utf_str = String::from_utf8(text.to_vec())?;
                DnsRecordData::Text(utf_str.into())
            }
            RecordType::SOA => DnsRecordData::SOA {
                mname: reader.read_qname()?,
                rname: reader.read_qname()?,
                serial: reader.read_u32()?,
                refresh: reader.read_u32()?,
                retry: reader.read_u32()?,
                expire: reader.read_u32()?,
                minimum: reader.read_u32()?,
            },
            RecordType::MX => DnsRecordData::MX {
                priority: reader.read_u16()?,
                host: reader.read_qname()?,
            },
            RecordType::SRV => DnsRecordData::SRV {
                priority: reader.read_u16()?,
                weight: reader.read_u16()?,
                port: reader.read_u16()?,
                target: reader.read_qname()?,
            },
            _ => {
                let raw_data = reader.read_bytes(data_length)?;
                DnsRecordData::Raw(raw_data.into())
            }
        })
    }
}

/// Represents a DNS record in a DNS message.
#[derive(Debug, Clone, Eq)]
pub struct DnsRecord {
    pub name: DomainName,
    pub record_type: RecordType,
    pub class: ClassType,
    pub ttl: u32,
    pub data: DnsRecordData,
}

impl DnsRecord {
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

impl DnsReadable for DnsRecord {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let name = reader.read_qname()?;
        let record_type = RecordType::try_from(reader.read_u16()?)?;
        let class = ClassType::try_from(reader.read_u16()?)?;
        let ttl = reader.read_u32()?;
        let data_length = reader.read_u16()? as usize;

        let data = DnsRecordData::read_from_record_type(reader, &record_type, data_length)?;

        Ok(Self {
            name,
            record_type,
            class,
            ttl,
            data,
        })
    }
}

impl DnsWritable for DnsRecord {
    fn write_to(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        writer.write_qname(&self.name)?;
        writer.write_u16(self.record_type as u16)?;
        writer.write_u16(self.class as u16)?;
        writer.write_u32(self.ttl)?;

        let rdlen_pos = writer.position();

        // Reserve rdlen so we can go back once we know the size.
        writer.write_u16(0)?;

        let before = writer.position();
        self.data.write(writer)?;
        let after = writer.position();
        let rdlen = (after - before) as u16;

        // Write the rdlen.
        writer.overwrite_bytes(rdlen_pos, &rdlen.to_be_bytes())?;

        Ok(())
    }
}

impl PartialEq for DnsRecord {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.record_type == other.record_type
            && self.class == other.class
            && self.ttl == other.ttl
            && self.data == other.data
    }
}

/// Represents EDNS (Extension Mechanisms for DNS) information in a DNS message.
#[derive(Debug, Clone, PartialEq)]
pub struct Edns {
    /// Max UDP payload size sender can handle
    pub udp_payload_size: u16,
    /// High bits of RCODE (ttl[31:24])
    extended_rcode: u8,
    /// EDNS version - must be 0.
    pub version: u8,
    /// Z flags
    z_flags: u16,
    /// Edns option
    pub options: Vec<EdnsOption>,
}

impl Default for Edns {
    fn default() -> Self {
        Self {
            udp_payload_size: 4096,
            extended_rcode: 0,
            version: 0,
            z_flags: 0,
            options: vec![],
        }
    }
}

impl Edns {
    // Get the do bit
    pub fn do_bit(&self) -> bool {
        self.z_flags & 0x8000 != 0
    }

    // Set the do bit
    pub fn set_do_bit(&mut self, v: bool) {
        if v {
            self.z_flags |= 0x8000;
        } else {
            self.z_flags &= !0x8000
        }
    }
}

impl DnsReadable for Edns {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let udp_payload_size = reader.read_u16()?;

        // TTL packed: ext_rcode | version | z_flags
        let ttl = reader.read_u32()?;

        let extended_rcode = ((ttl >> 24) & 0xFF) as u8;
        let version = ((ttl >> 16) & 0xFF) as u8;
        let z_flags = (ttl & 0xFFFF) as u16;

        // RDLEN + options;
        let rdlen = reader.read_u16()? as usize;
        let opts_end = reader.position() + rdlen;

        let mut options: Vec<EdnsOption> = Vec::new();

        while reader.position() < opts_end {
            let option = EdnsOption::read_from(reader)?;
            options.push(option);
        }
        Ok(Self {
            udp_payload_size,
            extended_rcode,
            version,
            z_flags,
            options,
        })
    }
}

impl DnsWritable for Edns {
    fn write_to(&self, writer: &mut DnsMessageWriter) -> anyhow::Result<()> {
        todo!()
    }
}

/// EDNS option
#[derive(Debug, Clone, PartialEq)]
pub struct EdnsOption {
    /// EDNS option code
    code: EdnsOptionCode,
    /// EDNS option length.
    len: u16,
    /// EDNS option data
    data: EdnsOptionData,
}

/// EDNS Option codes
///
/// Based on: https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-11
#[derive(Debug, Clone, PartialEq, TryFromPrimitive)]
#[repr(u16)]
pub enum EdnsOptionCode {
    /// Apple's DNS Long-Lived Queries Protocol (RFC 8764)
    LLQ = 1,
    /// Update Lease (RFC 9664)
    UpdateLease = 2,
    /// DNS Name Server Identifier (NSID) Option
    NSID = 3,
    /// DNSSEC Algorithm Understood (RFC 6975)
    DAU = 5,
    /// DNSSEC Hash Understood (RFC 6975)
    DHU = 6,
    /// NSEC3 Hash Understood (RFC 6975)
    N3U = 7,
    /// Client Subnet in DNS Queries (RFC 7871)
    ClientSubnet = 8,
    /// EDNS expire (RFC 7314)
    Expire = 9,
    /// EDNS Cookie (RFC 7873)
    Cookie = 10,
    /// EDNS TCP Keep Alive (RFC 7828)
    TcpKeepAlive = 11,
    /// EDNS Padding (7830)
    Padding = 12,
    /// CHAIN (RFC 7901)
    CHAIN = 13,
    /// EDNS Key Tag (RFC 8145)
    KeyTag = 14,
    /// Extended DNS error (RFC 8914)
    ExtendedDnsError = 15,
    /// Report channel (RFC 9567)
    ReportChannel = 18,
    /// Zone version (RFC 9660)
    ZONEVERSION = 19,

    /// Unknown or reserved/unassigned
    #[num_enum(alternatives = [16, 17, 21..=65536])]
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdnsOptionData {
    /// Lease
    Lease {
        /// Desired lease duration (Lease Update Request) or granted lease duration (Lease Update response), in seconds
        lease: u32,
        /// Optional desired (or granted) lease duration for KEY RRs, in seconds
        key_lease: Option<u32>,
    },
    /// Client Subnet
    ClientSubnet {
        family: u16,
        source_prefix: u8,
        scope_prefix: u8,
        address: Vec<u8>,
    },

    // Timeout in units of 100ms.
    Timeout(Option<u16>),

    // Paddingk
    Padding(u16),

    // Domain Name
    DomainName(DomainName),

    // Extended Dns Error
    ExtendedError {
        info_code: ExtendedDnsErrorInfoCode,
        extra_text: Option<String>,
    },

    /// Zone version Query
    ZoneVersionQuery,

    // Zone Version
    ZoneVersion {
        label_count: u8,
        r#type: u8,
        version: Vec<u8>,
    },

    /// Empty data
    Empty,

    // Raw data
    Raw(Vec<u8>),
}

impl EdnsOptionData {
    pub fn read(reader: &mut DnsMessageReader, code: &EdnsOptionCode, len: u16) -> anyhow::Result<Self> {
        Ok(match *code {
            EdnsOptionCode::ClientSubnet => {
                anyhow::ensure!(len >= 4, "ECS option too short (must be at least 4 bytes)");
                let family_bytes = reader.read_bytes(2)?;
                let source_prefix_length = reader.read_u8()?;
                let scope_prefix_length = reader.read_u8()?;
                let address_size = (source_prefix_length as usize).div_ceil(8);
                let address = reader.read_bytes(address_size)?;
                Self::ClientSubnet {
                    family: u16::from_be_bytes([family_bytes[0], family_bytes[1]]),
                    source_prefix: source_prefix_length,
                    scope_prefix: scope_prefix_length,
                    address: address.to_vec(),
                }
            }
            EdnsOptionCode::Cookie => Self::Raw(reader.read_bytes(len as usize)?.to_vec()),
            EdnsOptionCode::UpdateLease => {
                anyhow::ensure!(len == 4 || len == 8, "invalid UPDATE-LEASE option length: {}", len);
                let lease = reader.read_u32()?;
                let key_lease: Option<u32> = if len == 8 { Some(reader.read_u32()?) } else { None };
                Self::Lease { lease, key_lease }
            }
            EdnsOptionCode::DAU | EdnsOptionCode::DHU | EdnsOptionCode::N3U => {
                Self::Raw(reader.read_bytes(len as usize)?.to_vec())
            }
            EdnsOptionCode::TcpKeepAlive => {
                anyhow::ensure!(len == 0 || len == 2, "invalid TCP Keepalive option length: {}", len);

                // if len is 2 => present
                let timeout: Option<u16> = if len == 2 { Some(reader.read_u16()?) } else { None };
                Self::Timeout(timeout)
            }
            EdnsOptionCode::Padding => {
                reader.read_bytes(len as usize)?; // discard padding bytes
                Self::Padding(len)
            }
            EdnsOptionCode::CHAIN | EdnsOptionCode::ReportChannel => {
                if len == 0 {
                    return Ok(Self::Empty);
                }
                Self::DomainName(reader.read_qname_uncompressed(len as usize)?)
            }
            EdnsOptionCode::ExtendedDnsError => {
                let len_usize = usize::from(len);
                anyhow::ensure!(
                    len_usize >= std::mem::size_of::<u16>(),
                    "extended dns error length too short"
                );
                let info_code = ExtendedDnsErrorInfoCode::try_from(reader.read_u16()?)?;

                let remaining_length = len_usize - std::mem::size_of::<u16>();

                let remaining_bytes = reader.read_bytes(remaining_length)?;

                let extra_text: Option<String> = if !remaining_bytes.is_empty() {
                    Some(String::from_utf8_lossy(remaining_bytes).into_owned())
                } else {
                    None
                };

                Self::ExtendedError { info_code, extra_text }
            }
            EdnsOptionCode::ZONEVERSION => {
                // Query
                if len == 0 {
                    Self::ZoneVersionQuery
                    // Response
                } else {
                    anyhow::ensure!(len >= 2, "ZONEVERSION option too short: len={} (need at least 2)", len);

                    let label_count = reader.read_u8()?;
                    let typ = reader.read_u8()?;

                    let remaining_length = len as usize - (std::mem::size_of::<u8>() * 2);
                    let version_bytes = reader.read_bytes(remaining_length)?;

                    // todo: make this more concrete (version type parsing)

                    Self::ZoneVersion {
                        label_count,
                        r#type: typ,
                        version: version_bytes.to_vec(),
                    }
                }
            }
            _ => {
                let raw_data = reader.read_bytes(len as usize)?;
                Self::Raw(raw_data.into())
            }
        })
    }
}

impl DnsReadable for EdnsOption {
    fn read_from(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let code = EdnsOptionCode::try_from(reader.read_u16()?)?;
        let len = reader.read_u16()?;
        // todo: find a way to implement the DnsReader trait for EdnsOptionData. we can't currently because it relies on the code which we parse here.
        let data = EdnsOptionData::read(reader, &code, len)?;
        Ok(Self {
            code: code,
            len,
            data: data,
        })
    }
}

/// Extended DNS error info code
#[derive(Debug, Clone, Copy, TryFromPrimitive, PartialEq)]
#[repr(u16)]
pub enum ExtendedDnsErrorInfoCode {
    /// The error in question falls into a category that does not match known extended error codes.
    OtherError = 0,
    /// The resolver attempted to perform DNSSEC validation, but a DNSKEY RRset contained only unsupported DNSSEC algorithms.
    UnsupportedDnskeyAlgorithm = 1,
    ///The resolver attempted to perform DNSSEC validation, but a DS RRset contained only unsupported Digest Types.
    UnsupportedDsDigestType = 2,
    /// The resolver was unable to resolve the answer within its time limits and decided to answer with previously cached data instead of answering with an error. This is typically caused by problems communicating with an authoritative server, possibly as result of a denial of service (DoS) attack against another network.
    StaleAnswer = 3,
    /// For policy reasons (legal obligation or malware filtering, for instance), an answer was forged. Note that this should be used when an answer is still provided, not when failure codes are returned instead.
    ForgedAnswer = 4,
    /// The resolver attempted to perform DNSSEC validation, but validation ended in the Indeterminate state
    DnssecIndeterminate = 5,
    /// The resolver attempted to perform DNSSEC validation, but validation ended in the Bogus state.
    DnssecBogus = 6,
    /// The resolver attempted to perform DNSSEC validation, but no signatures are presently valid and some (often all) are expired.
    SignatureExpired = 7,
    /// The resolver attempted to perform DNSSEC validation, but no signatures are presently valid and at least some are not yet valid.
    SignatureNotYetValid = 8,
    /// A DS record existed at a parent, but no supported matching DNSKEY record could be found for the child.
    DnsKeyMissing = 9,
    /// The resolver attempted to perform DNSSEC validation, but no RRSIGs could be found for at least one RRset where RRSIGs were expected.
    RrSigsMissing = 10,
    /// The resolver attempted to perform DNSSEC validation, but no Zone Key Bit was set in a DNSKEY.
    NoZoneKeyBitSet = 11,
    /// The resolver attempted to perform DNSSEC validation, but the requested data was missing and a covering NSEC or NSEC3 was not provided.
    NSecMissing = 12,
    /// The resolver is returning the SERVFAIL RCODE from its cache.
    CachedError = 13,
    /// The server is unable to answer the query, as it was not fully functional when the query was received.
    NotReady = 14,
    /// The server is unable to respond to the request because the domain is on a blocklist due to an internal security policy imposed by the operator of the server resolving or forwarding the query.
    Blocked = 15,
    /// The server is unable to respond to the request because the domain is on a blocklist due to an external requirement imposed by an entity other than the operator of the server resolving or forwarding the query. Note that how the imposed policy is applied is irrelevant (in-band DNS filtering, court order, etc.).
    Censored = 16,
    /// The server is unable to respond to the request because the domain is on a blocklist as requested by the client. Functionally, this amounts to "you requested that we filter domains like this one."
    Filtered = 17,
    /// An authoritative server or recursive resolver that receives a query from an "unauthorized" client can annotate its REFUSED message with this code. Examples of "unauthorized" clients are recursive queries from IP addresses outside the network, blocklisted IP addresses, local policy, etc.
    Prohibited = 18,
    /// The resolver was unable to resolve an answer within its configured time limits and decided to answer with a previously cached NXDOMAIN answer instead of answering with an error. This may be caused, for example, by problems communicating with an authoritative server, possibly as result of a denial of service (DoS) attack against another network.
    StaleNxDomainAnswer = 19,
    /// An authoritative server that receives a query with the Recursion Desired (RD) bit clear, or when it is not configured for recursion for a domain for which it is not authoritative, SHOULD include this EDE code in the REFUSED response. A resolver that receives a query with the RD bit clear SHOULD include this EDE code in the REFUSED response.
    NotAuthorative = 20,
    /// The requested operation or query is not supported.
    NotSupported = 21,
    /// The resolver could not reach any of the authoritative name servers (or they potentially refused to reply).
    NoReachableAuthority = 22,
    /// An unrecoverable error occurred while communicating with another server.
    NetworkError = 23,
    /// The authoritative server cannot answer with data for a zone it is otherwise configured to support. Examples of this include its most recent zone being too old or having expired.
    InvalidData = 24,
}

#[cfg(test)]
mod tests {

    use crate::builder::DnsMessageBuilder;

    use super::*;

    #[test]

    fn test_writer_reader() {
        let qname = DomainName::from_ascii("example.com").unwrap();

        let mut writer = DnsMessageWriter::new();
        writer.write_u8(42).unwrap();
        writer.write_u16(12345).unwrap();
        writer.write_u32(67890).unwrap();
        writer.write_qname(&qname).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = DnsMessageReader::new(&bytes);
        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.read_u16().unwrap(), 12345);
        assert_eq!(reader.read_u32().unwrap(), 67890);
        assert_eq!(reader.read_qname().unwrap(), qname);
    }

    #[test]
    fn test_encode_decode() {
        let packet = DnsMessageBuilder::new()
            .with_id(12345)
            .with_flags(DnsFlags {
                response: true,
                opcode: DnsOpcode::Query,
                authorative_answer: false,
                truncated: false,
                recursion_desired: true,
                recursion_available: false,
                z: false,
                authentic_data: false,
                checking_disabled: false,
                rcode_low: 0,
            })
            .add_question(DnsQuestion {
                qname: DomainName::from_ascii("example.com").unwrap(),
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
        assert!(decoded_message.flags.response);
        assert_eq!(decoded_message.flags.opcode, DnsOpcode::Query);
    }
    #[test]
    fn test_ns_query_encoding() {
        let domain_name = DomainName::from_ascii("com").unwrap();
        let packet = DnsMessageBuilder::new()
            .add_question(DnsQuestion::new(domain_name, RecordType::NS, ClassType::IN))
            .build();

        let encoded = packet.encode().unwrap();
        let hex: Vec<String> = encoded.iter().map(|b| format!("{:02x}", b)).collect();
        println!("Packet: {}", hex.join(" "));

        assert!(hex.contains(&"00".to_string()));
        assert!(hex.contains(&"02".to_string())); // QTYPE = 2
        assert!(hex.contains(&"00".to_string()));
        assert!(hex.contains(&"01".to_string())); // QCLASS = 1
    }

    #[test]
    fn test_message_compression() {
        let message = DnsMessageBuilder::new()
            .with_id(1)
            .with_flags(DnsFlags::default())
            .add_question(DnsQuestion::new(
                DomainName::from_user("google.com").unwrap(),
                RecordType::A,
                ClassType::IN,
            ))
            .add_question(DnsQuestion::new(
                DomainName::from_user("mail.google.com").unwrap(),
                RecordType::A,
                ClassType::IN,
            ))
            .build();

        let encoded = message.encode().unwrap();
        let decoded = DnsMessage::decode(&encoded).unwrap();

        assert!(message == decoded);
    }
}
