use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use libtpass::errors::TPassError;
use reqwest::Error;
use serde::{Deserialize, Serialize};

use serde_json::{json, Value};
//----- A better error strategy
//top level, our domain specific errors will be transformed to AppErrors for retunning to users
use libfr::errors::FRError;

#[derive(Debug)]
pub enum AppError {
    InvalidInput(String), //for very simple messages or an error we're not sure how to format yet.
    Generic(String),      //for very simple messages or an error we're not sure how to format yet.
    Standard(StandardError), //standard json error message. the http call was good but there was an api failure.
}

impl From<TPassError> for AppError {
    fn from(te: TPassError) -> Self {
        let msg = te.to_string();
        let gen_code = "TPASS_ERR".to_string();
        match te {
            TPassError::ClientNotFound { .. } => AppError::Standard(StandardError {
                code: "TPASS_CLIENT_NOT_FOUND".into(),
                message: msg,
                details: None,
            }),
            TPassError::Generic(s) => {
                AppError::Standard(StandardError { code: gen_code, message: s, details: None })
            }
            TPassError::Http(_) => {
                AppError::Standard(StandardError { code: gen_code, message: msg, details: None })
            }
            TPassError::Json(_) => {
                AppError::Standard(StandardError { code: gen_code, message: msg, details: None })
            }

            TPassError::MissingImage { last_name, first_name, ext_id, img_url } => {
                AppError::Standard(StandardError {
                    code: gen_code,
                    message: msg,
                    details: Some(json!({
                        "last_name": last_name,
                        "first_name": first_name,
                        "ext_id": ext_id,
                        "img_url": img_url,
                    })),
                })
            }

            TPassError::MissingImageURL { last_name, first_name, ext_id } => {
                AppError::Standard(StandardError {
                    code: gen_code,
                    message: msg,
                    details: Some(json!({
                        "last_name": last_name,
                        "first_name": first_name,
                        "ext_id": ext_id,
                    })),
                })
            }

            TPassError::RegisterEnrollment { value, .. } => AppError::Standard(StandardError {
                code: gen_code,
                message: msg,
                details: Some(value),
            }),
        }
    }
}

//convert from library errors to App level errors.
impl From<FRError> for AppError {
    fn from(fe: FRError) -> Self {
        match fe {
            FRError::Generic { code, message, details } => {
                AppError::Standard(StandardError { code, message, details })
            }
            //we might want to change message if it has internal details not meaningful to user
            FRError::Remote(msg) => AppError::Standard(StandardError {
                code: "REMOTE_ERR".into(),
                message: msg,
                details: None,
            }),
            FRError::Duplicate { ext_id, fr_id, score } => AppError::Standard(StandardError {
                code: "DUPLICATE_ERR".into(),
                message: "an enrollment already exists that matches face".into(),
                details: Some(json!({
                    "ext_id": ext_id,
                    "fr_id": fr_id,
                    "score": score
                })),
            }),
            FRError::CreateIdentity { ext_id } => AppError::Standard(StandardError {
                code: "CREATE_IDENTITY_ERR".into(),
                message: "enrollment failed to create identity".into(),
                details: Some(json!({ "ext_id": ext_id})),
            }),
            FRError::AddFace { fr_id } => AppError::Standard(StandardError {
                code: "ADD_FACE_ERR".into(),
                message: "could not add face for identity".into(),
                details: Some(json!({ "fr_id": fr_id})),
            }),
            FRError::MissingImage { fr_id } => AppError::Standard(StandardError {
                code: "MISSING_IMAGE_ERR".into(),
                message: "facial recognition failed. no image found".into(),
                details: Some(json!({ "fr_id": fr_id})),
            }),
            FRError::PoorQuality { quality, min_quality } => AppError::Standard(StandardError {
                code: "QUALITY_LOW_ERR".into(),
                message: "image quality did not meet standard".into(),
                details: Some(json!({ "quality": quality, "min_quality": min_quality})),
            }),
            FRError::CreateEnrollment { ext_id, message } => AppError::Standard(StandardError {
                code: "CREATE_ENROLLMENT_ERR".into(),
                message,
                details: Some(json! ({"ext_id": ext_id})),
            }),
            FRError::DeleteEnrollment { fr_id, message } => AppError::Standard(StandardError {
                code: "DELETE_ENROLLMENT_ERR".into(),
                message,
                details: Some(json! ({ "fr_id": fr_id})),
            }),
            FRError::SaveProfile { message, .. } => AppError::Standard(StandardError {
                code: "SAVE_PROFILE_ERR".into(),
                message,
                details: None,
            }),
            FRError::FaceNotFound => AppError::Standard(StandardError {
                code: "FACE_NOT_FOUND_ERR".into(),
                message: "No faces were detected in image".into(),
                details: None,
            }),
            FRError::Engine(msg) => AppError::Standard(StandardError {
                code: "GENERIC_FR_ERR".into(),
                message: msg,
                details: None,
            }),
            FRError::Repo(msg) => AppError::Standard(StandardError {
                code: "REPO_ERR".into(),
                message: msg,
                details: None,
            }),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(re: Error) -> Self {
        //TODO: match on status code when needed?
        AppError::Generic(re.to_string())
    }
}

//NOTE: We return 200 on our errors if they are api based. Opinion: Json apis are not actually REST and HTTP is just a transport.
//We should stop pretending like we're following HTTP properly. . Let's let it go.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AppError::Standard(ce) => {
                (StatusCode::OK, Json(ce)) //passing through the whole thing. we maty want to transform into something else
            }
            //These are just simple messages, we convert them to standard error with a default code and no detail.
            AppError::Generic(msg) => {
                let std_err =
                    StandardError { code: "GENERIC_ERR".into(), message: msg, details: None };
                (StatusCode::OK, Json(std_err))
            }
            AppError::InvalidInput(msg) => {
                let std_err =
                    StandardError { code: "INVALID_INPUT".into(), message: msg, details: None };
                (StatusCode::OK, Json(std_err))
            }
        };

        (status, error).into_response()
    }
}

//a general format for returning most api based errors to client.
#[derive(Serialize, Deserialize, Debug)]
pub struct StandardError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}
