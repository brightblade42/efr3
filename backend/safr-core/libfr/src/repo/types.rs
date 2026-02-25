use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{RepoError, RepoResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExternalId(String);

impl ExternalId {
    pub fn new(value: impl Into<String>) -> RepoResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(RepoError::message("external id cannot be empty"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for ExternalId {
    type Error = RepoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ExternalId {
    type Error = RepoError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}

impl From<ExternalId> for String {
    fn from(value: ExternalId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRecord {
    pub external_id: ExternalId,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub image_url: Option<String>,
    pub raw_data: Option<Value>,
    pub fr_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRecord {
    pub external_id: ExternalId,
    pub data: Vec<u8>,
    pub size: Option<f32>,
    pub url: Option<String>,
    pub quality: f32,
    pub acceptability: f32,
    pub raw_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationErrorRecord {
    pub external_id: Option<ExternalId>,
    pub fr_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentLogRecord {
    pub id: i64,
    pub code: String,
    pub payload: Value,
    pub retry_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentMetadataRecord {
    pub profiles_total: i64,
    pub profiles_with_fr_id: i64,
    pub images_total: i64,
    pub registration_errors_total: i64,
    pub enrollment_logs_total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentResetRecord {
    pub profiles_deleted: i64,
    pub images_deleted: i64,
    pub registration_errors_deleted: i64,
    pub enrollment_logs_deleted: i64,
}
