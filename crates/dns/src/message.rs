use std::{
    hash::Hash,
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use bytes::Bytes;

use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

use crate::{domain_name::DomainName, reader::DnsMessageReader, writer::DnsMessageWriter};

/// Represents a DNS message.
/// This struct encapsulates the various components of a DNS message,
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
        let flags = DnsFlags::try_from(reader.read_u16()?)?;

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

    pub fn rcode(&self) -> anyhow::Result<DnsResponseCode> {
        let low = self.flags.rcode_low as u16;
        let high = self.edns.as_ref().map(|e| e.extended_rcode).unwrap_or(0) as u16;
        let code = DnsResponseCode::try_from((high << 4) | low)?;
        Ok(code)
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

impl TryFrom<u16> for DnsFlags {
    type Error = anyhow::Error;

    fn try_from(bytes: u16) -> Result<Self, Self::Error> {
        Ok(Self {
            qr: (bytes >> 15) & 0x1 != 0,
            opcode: DnsOpcode::try_from(((bytes >> 11) & 0xF) as u8)?,
            aa: (bytes >> 10) & 0x1 != 0,
            tc: (bytes >> 9) & 0x1 != 0,
            rd: (bytes >> 8) & 0x1 != 0,
            ra: (bytes >> 7) & 0x1 != 0,
            z: (bytes >> 6) & 0x1 != 0,
            ad: (bytes >> 5) & 0x1 != 0,
            cd: (bytes >> 4) & 0x1 != 0,
            rcode_low: (bytes & 0x0F) as u8,
        })
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
        Self {
            qname,
            qtype,
            qclass,
        }
    }

    /// Create a new DNS question from a reader.
    pub fn read(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let qname = reader.read_qname()?;
        let qtype = RecordType::try_from(reader.read_u16()?)?;
        let qclass = ClassType::try_from(reader.read_u16()?)?;

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

/// DNS record types.
///
/// Based on: https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-4
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, IntoPrimitive)]
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

    /// Convert the DNS record data to a string representation, if applicable.
    pub fn to_str(&self) -> Option<String> {
        match self {
            DnsRecordData::Raw(_) => None,
            DnsRecordData::Ipv4(addr) => Some(addr.to_string()),
            DnsRecordData::Ipv6(addr) => Some(addr.to_string()),
            DnsRecordData::Text(text) => Some(text.to_string()),
            DnsRecordData::DomainName(name) => Some(name.to_string()),
            DnsRecordData::SOA { .. } => None,
            DnsRecordData::MX { .. } => None,
            DnsRecordData::SRV { .. } => None,
        }
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
    /// Create a new DNS record.
    pub fn read(reader: &mut DnsMessageReader) -> anyhow::Result<Self> {
        let name = reader.read_qname()?;
        let r#type = RecordType::try_from(reader.read_u16()?)?;
        let class = ClassType::try_from(reader.read_u16()?)?;
        let ttl = reader.read_u32()?;
        let data_length = reader.read_u16()? as usize;

        let data = match r#type {
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
            RecordType::TXT => {
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
        assert!(decoded_message.flags.qr);
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
}
