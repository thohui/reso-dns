use crate::database::DatabaseError;

pub mod blocklist;
pub mod config;
pub mod local_records;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    NotFound(String),

    #[error("internal error")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}
