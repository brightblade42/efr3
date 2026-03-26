use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileRecord {
    pub ext_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub img_url: Option<String>,
    pub raw_data: Option<Value>,
    pub fr_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImageRecord {
    pub ext_id: String,
    pub data: Vec<u8>,
    pub size: Option<f32>,
    pub url: Option<String>,
    pub quality: f32,
    pub acceptability: f32,
    pub raw_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegistrationErrorRecord {
    pub ext_id: Option<String>,
    pub fr_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnrollmentLogRecord {
    pub id: i64,
    pub code: String,
    pub error: Value,
    pub input: Value,
    //pub retry_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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
