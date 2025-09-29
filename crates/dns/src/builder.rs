use super::message::{DnsFlags, DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode};

/// Builder
#[derive(Debug, Clone, Default)]
pub struct DnsMessageBuilder {
    id: u16,
    flags: DnsFlags,
    questions: Vec<DnsQuestion>,
    answers: Vec<DnsRecord>,
    authority_records: Vec<DnsRecord>,
    additional_records: Vec<DnsRecord>,
    response_code: Option<DnsResponseCode>,
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
            response_code: None,
        }
    }

    /// Set the ID for the DNS packet.
    pub fn with_id(mut self, id: u16) -> Self {
        self.id = id;
        self
    }

    pub fn with_questions(mut self, questions: Vec<DnsQuestion>) -> Self {
        self.questions = questions;
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
        self.response_code = Some(response_code);
        self
    }

    pub fn build(self) -> DnsMessage {
        let flags = if let Some(rcode) = self.response_code {
            let mut f = self.flags;
            f.qr = true;
            f.rcode_low = rcode.into();
            f
        } else {
            self.flags
        };

        DnsMessage::new(
            self.id,
            flags,
            self.questions,
            self.answers,
            self.authority_records,
            self.additional_records,
        )
    }
}
