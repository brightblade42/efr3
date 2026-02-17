pub mod adapters;
pub mod domain;

use crate::FRError;
use thiserror::Error;

pub type V2Result<T> = Result<T, V2Error>;

#[derive(Debug, Error)]
pub enum V2Error {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Fr(#[from] FRError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl V2Error {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
