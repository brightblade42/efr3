use libpv::errors::PVApiError;
use libtpass::errors::TPassError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tracing::error;
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

    #[error("🟡 Quality too low: {quality:.2}")]
    PoorQuality { quality: f32 },

    #[error("😬 Add Face failed for identity: {fr_id})")]
    AddFace { fr_id: String },

    #[error("😬 Add Face failed for identity: {fr_id})")]
    MissingImage { fr_id: String },

    #[error(" Remote: {0})")]
    Remote(String),
    #[error("🔴 Paravision error: {0}")]
    Engine(String),
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

// impl From<FaceError> for FRError {
//     fn from(err: FaceError) -> Self {
//         match err {
//             FaceError::Duplicate { ref ext_id, ref fr_id, score } => Self::with_details(
//                 409,
//                 "duplicate_found",
//                 &err.to_string(),
//                 json!({ "ext_id": ext_id, "fr_id": fr_id, "score": score }),
//             ),
//             FaceError::PoorQuality { quality } => Self::with_details(
//                 400,
//                 "poor_quality",
//                 &err.to_string(),
//                 json!({ "quality": quality }),
//             ),
//             FaceError::Engine(msg) => Self::with_code(500, "engine_failure", &msg),
//         }
//     }
// }

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
        // match e {
        //     TPassError::Generic(msg) => Self::TPass(msg),
        //     TPassError::Http(e) => Self::TPass(format!("Http: {}", e)),
        //     TPassError::JsonError(e) => Self::TPass(format!("Json: {}", e)),
        // }

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
