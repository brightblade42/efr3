//! Repository layer for local Postgres-backed enrollment data.
//!
//! The repository stores and retrieves local state that complements the FR engine and remote
//! system of record, such as profile snapshots, match logs, enrollment error logs, and admin
//! summary counts.

pub mod sqlx;
pub mod types;

pub use self::sqlx::SqlxFrRepository;
pub use self::types::{
    EnrollmentLogRecord, EnrollmentMetadataRecord, EnrollmentResetRecord, ImageRecord,
    ProfileRecord, RegistrationErrorRecord,
};

use thiserror::Error;

pub type RepoResult<T> = Result<T, RepoError>;

/// Repository error wrapper for SQL and JSON conversion failures.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Sqlx(#[from] ::sqlx::Error),
    #[error(transparent)]
    Json(#[from] ::serde_json::Error),
}

impl RepoError {
    /// Create a simple message-based repository error.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
