use libpv::errors::PVApiError;
use libtpass::errors::TPassError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tracing::error;

use crate::repo::RepoError;
#[derive(Serialize, Deserialize, Debug, Error)]
#[error("{message}")]
pub struct FRError2 {
    pub code: u16,
    pub name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum FRError {
    #[error("🎯 Duplicate found: {ext_id} | {fr_id} (score: {score:.4})")]
    Duplicate { ext_id: String, fr_id: String, score: f32 },

    #[error("🟡 Quality too low: min_qualutyL {min_quality:.2} quality: {quality:.2}")]
    PoorQuality { quality: f32, min_quality: f32 },

    #[error("😬 Add Face failed for identity: fri_id: {fr_id})")]
    AddFace { fr_id: String },

    #[error("😬 No faces were detected in image")]
    FaceNotFound,
    #[error("😬 Add Face failed for identity: fr_id: {fr_id})")]
    MissingImage { fr_id: String },

    #[error("🔴 Create identity failed: ext_id: {ext_id})")]
    CreateIdentity { ext_id: String },

    #[error("🔴 Failed to save profile: for ext_id: {ext_id} msg: {message})")]
    SaveProfile { ext_id: String, message: String },
    #[error("🔴 Create enrollment error: ext_id: {ext_id} msg: {message})")]
    CreateEnrollment { ext_id: String, message: String },
    #[error("🔴 Delete enrollment error: fr_id: {fr_id} msg: {message})")]
    DeleteEnrollment { fr_id: String, message: String },
    #[error(" Remote: {0})")]
    Remote(String),
    #[error("🔴 Paravision error: {0}")]
    Engine(String),
    #[error("🔴 Repo error: {0}")]
    Repo(String),
    #[error("🟣 generic error: {code}  | {message}")]
    Generic {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
    },
}

// impl FRError {
//     pub fn new() -> Self {
//         Self {
//             code: 500,
//             name: "generic_error".to_string(),
//             message: "could not perform fr operation. this is a catch all.".to_string(),
//             details: None,
//         }
//     }
//     pub fn with_code(code: u16, name: &str, message: &str) -> Self {
//         Self { code, name: name.to_string(), message: message.to_string(), details: None }
//     }

//     pub fn with_details(code: u16, name: &str, message: &str, details: Value) -> Self {
//         Self { code, name: name.to_string(), message: message.to_string(), details: Some(details) }
//     }
// }

// impl Default for FRError {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// #[derive(Debug, Error)]
// pub enum FaceError {
//     #[error("Duplicate found: {ext_id} | {fr_id} (score: {score:.4})")]
//     Duplicate { ext_id: String, fr_id: String, score: f32 },

//     #[error("Quality too low: {quality:.2}")]
//     PoorQuality { quality: f32 },

//     #[error("Paravision error: {0}")]
//     Engine(String),
// }
impl From<RepoError> for FRError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Json(e) => {
                error!("‼️ Repo error: {}", e);
                FRError::Repo(e.to_string())
            }
            RepoError::Message(msg) => {
                error!("‼️ Repo error: {}", msg);
                FRError::Repo(msg)
            }
            RepoError::Sqlx(e) => {
                error!("‼️ Repo error: {}", e);
                FRError::Repo(e.to_string())
            }
        }
    }
}
impl From<PVApiError> for FRError {
    fn from(pv: PVApiError) -> Self {
        //TODO: update PVApiError to provide name. we might not even
        // need that error anymore
        FRError::Generic { code: "PV_API_ERR".to_string(), message: pv.message, details: None }
    }
}

impl From<&PVApiError> for FRError {
    fn from(pv: &PVApiError) -> Self {
        FRError::Generic {
            code: "PV_API_ERR".to_string(),
            message: pv.message.clone(),
            details: None,
        }
    }
}

impl From<TPassError> for FRError {
    fn from(e: TPassError) -> Self {
        match e {
            TPassError::Generic(msg) => {
                error!(target: "remote_integration", "🆔 {}", msg);

                Self::Remote(msg)
            }
            TPassError::RegisterEnrollment { ext_id, value } => {
                let msg = format!("Registration failed for {}: {}", ext_id, value);

                // Log it immediately to your RHEL terminal
                error!(target: "remote_integration", "🆔 {}", msg);

                Self::Remote(msg)
            }
            // Fallback for HttpError, JsonError, etc.
            _ => {
                let msg = e.to_string();
                error!(target: "remote_integration", "🆔 {}", msg);
                Self::Remote(msg)
            }
        }
    }
}
impl From<SqlxError> for FRError {
    fn from(se: SqlxError) -> Self {
        Self::Generic { code: "SQLX_ERROR".to_string(), message: se.to_string(), details: None }
    }
}

impl From<serde_json::Error> for FRError {
    fn from(se: serde_json::Error) -> Self {
        FRError::Generic { code: "JSON_ERROR".to_string(), message: se.to_string(), details: None }
    }
}
