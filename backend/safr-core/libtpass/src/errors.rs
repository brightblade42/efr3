use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TPassError {
    #[error("{0}")]
    GenericError(#[source] Box<dyn StdError + Send + Sync>),
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("{0}")]
    DBError(#[source] Box<dyn StdError + Send + Sync>),
}
