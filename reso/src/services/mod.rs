use crate::database::DatabaseError;

pub mod config;
pub mod domain_rules;
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
    Internal(#[from] anyhow::Error),
}

impl From<DatabaseError> for ServiceError {
    fn from(e: DatabaseError) -> Self {
        ServiceError::Internal(e.into())
    }
}
