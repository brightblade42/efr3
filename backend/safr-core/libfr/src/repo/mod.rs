pub mod sqlx;
pub mod types;

pub use self::sqlx::SqlxFrRepository;
pub use self::types::{
    EnrollmentLogRecord, EnrollmentMetadataRecord, EnrollmentResetRecord, ExternalId, ImageRecord,
    ProfileRecord, RegistrationErrorRecord,
};

use thiserror::Error;

pub type RepoResult<T> = Result<T, RepoError>;

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
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
