use super::message::{
    DnsFlags, DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode, Edns,
};

/// Builder
#[derive(Debug, Clone, Default)]
pub struct DnsMessageBuilder {
    id: u16,
    flags: DnsFlags,
    questions: Vec<DnsQuestion>,
    answers: Vec<DnsRecord>,
    authority_records: Vec<DnsRecord>,
    additional_records: Vec<DnsRecord>,
}

impl DnsMessageBuilder {
    /// Create a new DNS message builder.
    pub fn new() -> Self {
        Self {
            id: 0,
            flags: DnsFlags {
                qr: false,
                opcode: DnsOpcode::Query,
                aa: false,
                tc: false,
                rd: true,
                ra: false,
                z: false,
                ad: false,
                cd: false,
                rcode_low: 0,
            },
            questions: Vec::new(),
            answers: Vec::new(),
            authority_records: Vec::new(),
            additional_records: Vec::new(),
        }
    }

    /// Set the ID for the DNS packet.
    pub fn with_id(mut self, id: u16) -> Self {
        self.id = id;
        self
    }

    /// Set the flags for the DNS packet.
    pub fn with_flags(mut self, flags: DnsFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Add a question to the DNS packet.
    pub fn add_question(mut self, question: DnsQuestion) -> Self {
        self.questions.push(question);
        self
    }

    /// Add an answer to the DNS packet.
    pub fn add_answer(mut self, answer: DnsRecord) -> Self {
        self.answers.push(answer);
        self
    }

    /// Add an authority record to the DNS packet.
    pub fn add_authority_record(mut self, record: DnsRecord) -> Self {
        self.authority_records.push(record);
        self
    }

    /// Add an additional record to the DNS packet.
    pub fn add_additional_record(mut self, record: DnsRecord) -> Self {
        self.additional_records.push(record);
        self
    }

    pub fn with_response(mut self, response_code: DnsResponseCode) -> Self {
        // todo: add edns support.
        let value: u16 = response_code.into();
        self.flags.rcode_low = (value & 0x0F) as u8;
        self
    }

    pub fn build(self) -> DnsMessage {
        DnsMessage::new(
            self.id,
            self.flags,
            self.questions,
            self.answers,
            self.authority_records,
            self.additional_records,
        )
    }
}
