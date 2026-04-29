use crate::DnsResponseCode;

/// Error that can occur during DNS message reading.
#[derive(Debug, thiserror::Error)]
pub enum DnsReadError {
    #[error("seek out of bounds: pos {pos} >= len {len}")]
    SeekOutOfBounds { pos: usize, len: usize },

    #[error("buffer underflow at pos {pos}: need {need} bytes, have {have}")]
    BufferUnderflow { pos: usize, need: usize, have: usize },

    #[error("compression pointer loop at offset {offset}")]
    CompressionLoop { offset: usize },

    #[error("compression pointer offset {offset} out of bounds (buf len {len})")]
    CompressionOutOfBounds { offset: usize, len: usize },

    #[error("compression pointer not allowed in uncompressed name (byte 0x{byte:02x})")]
    CompressionNotAllowed { byte: u8 },

    #[error("trailing bytes after name: pos {pos}, end {end}")]
    TrailingBytes { pos: usize, end: usize },

    #[error("empty label")]
    EmptyLabel,

    #[error("name exceeds 255 octets (wire format length: {len})")]
    NameTooLong { len: usize },

    #[error("label exceeds 63 octets: {len}")]
    LabelTooLong { len: usize },

    #[error("unterminated name: no root label within {len} bytes")]
    UnterminatedName { len: usize },

    #[error("multiple OPT records in additional section")]
    MultipleOpt,

    #[error("invalid IDNA domain: {input}: {cause}")]
    InvalidIdna { input: String, cause: idna::Errors },
}

/// Error that can occur during DNS message writing.
#[derive(Debug, thiserror::Error)]
pub enum DnsWriteError {
    #[error("buffer overflow: need {need} bytes, current len {current_len}, max {max_len}")]
    BufferOverflow {
        need: usize,
        current_len: usize,
        max_len: usize,
    },

    #[error("overwrite out of bounds: pos {pos}, len {len}, buf len {buf_len}")]
    OverwriteOutOfBounds { pos: usize, len: usize, buf_len: usize },
}

/// General error type for DNS processing errors.
#[derive(Debug, thiserror::Error)]
pub enum DnsError {
    #[error("invalid opcode {0}")]
    InvalidOpcode(u8),

    #[error("invalid option length for {option}: expected {expected} bytes, got {actual} bytes")]
    InvalidOptionLength {
        option: String,
        expected: usize,
        actual: usize,
    },

    #[error("multiple OPT records in additional section")]
    MultipleOptRecords,

    #[error("unknown address family: {family}")]
    UnknownAddressFamily { family: u16 },

    #[error("RDATA length overflow: {len} bytes exceeds u16")]
    RdataLengthOverflow { len: usize },

    #[error("EDNS version {0} not supported")]
    UnsupportedEdnsVersion(u8),

    #[error("ECS prefix {prefix} exceeds max {max} for family {family}")]
    EcsPrefixTooLarge { family: u16, prefix: u8, max: u8 },

    #[error(transparent)]
    Read(#[from] DnsReadError),

    #[error(transparent)]
    Write(#[from] DnsWriteError),
}

impl DnsError {
    /// Map the error to an appropriate DNS response code.
    pub fn response_code(&self) -> DnsResponseCode {
        match self {
            DnsError::InvalidOpcode(_) => DnsResponseCode::NotImp,
            DnsError::InvalidOptionLength { .. } => DnsResponseCode::FormatError,
            DnsError::UnknownAddressFamily { .. } => DnsResponseCode::FormatError,
            DnsError::Read(_) => DnsResponseCode::FormatError,
            DnsError::Write(_) => DnsResponseCode::ServerFailure,
            DnsError::RdataLengthOverflow { .. } => DnsResponseCode::FormatError,
            DnsError::UnsupportedEdnsVersion(_) => DnsResponseCode::FormatError,
            DnsError::EcsPrefixTooLarge { .. } => DnsResponseCode::FormatError,
            DnsError::MultipleOptRecords => DnsResponseCode::FormatError,
        }
    }
}

pub type ReadResult<T> = std::result::Result<T, DnsReadError>;
pub type WriteResult<T> = std::result::Result<T, DnsWriteError>;
pub type Result<T> = std::result::Result<T, DnsError>;
