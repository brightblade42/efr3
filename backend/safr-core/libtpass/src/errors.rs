use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TPassError {
    #[error("Generic Error: {0}")]
    Generic(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Missing imgUrl for: {last_name} {first_name} {ext_id}")]
    MissingImageURL { last_name: String, first_name: String, ext_id: u64 },
    #[error("Missing image for: {last_name} {first_name} {ext_id}")]
    MissingImage { last_name: String, first_name: String, ext_id: u64, img_url: String },
    #[error("Client not found for ccode: {ext_id}")]
    ClientNotFound { ext_id: u64 },
    #[error("Register enrollment failed for ccode: {ext_id}")]
    RegisterEnrollment { ext_id: u64, value: Value },
}
