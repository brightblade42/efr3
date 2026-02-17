//use std::any::TypeId;
//use std::backtrace::Backtrace;
use reqwest::{Error as ReqwestError, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tonic::{transport::Error as TonicTransportError, Code as TonicCode, Status as TonicStatus};

//Error from api service may be app errors contained in a good response, or
// an service level http error. We will treat them the same
//an api based error returned from paravision server
#[derive(Serialize, Deserialize, Debug, Error)]
#[error("{message}")]
pub struct PVApiError {
    pub code: u16,
    pub message: String,
    pub details: Option<Vec<String>>, //reserved for future use (PV docs)
}

impl PVApiError {
    pub fn new() -> Self {
        Self {
            code: 500, //default to a 500 until we know better
            message: "could not properly reach paravision api".to_string(),
            details: None,
        }
    }

    pub fn with_code(code: u16, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            details: None,
        }
    }
}

impl Default for PVApiError {
    fn default() -> Self {
        Self::new()
    }
}

//when we receive an http level error rather than an api level error
impl From<ReqwestError> for PVApiError {
    fn from(e: ReqwestError) -> Self {
        let stat: StatusCode = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut pv_err = PVApiError::new();
        pv_err.code = stat.as_u16();
        pv_err.message = e.to_string();
        pv_err
    }
}

impl From<serde_json::Error> for PVApiError {
    fn from(e: serde_json::Error) -> Self {
        let mut pv_err = PVApiError::new();
        pv_err.message = e.to_string();
        pv_err
    }
}

impl From<TonicStatus> for PVApiError {
    fn from(status: TonicStatus) -> Self {
        let mut pv_err = PVApiError::new();
        pv_err.code = tonic_code_to_http(status.code());
        pv_err.message = status.message().to_string();
        pv_err
    }
}

impl From<TonicTransportError> for PVApiError {
    fn from(err: TonicTransportError) -> Self {
        let mut pv_err = PVApiError::new();
        pv_err.code = 503;
        pv_err.message = err.to_string();
        pv_err
    }
}

fn tonic_code_to_http(code: TonicCode) -> u16 {
    match code {
        TonicCode::Ok => 200,
        TonicCode::Cancelled => 499,
        TonicCode::Unknown => 500,
        TonicCode::InvalidArgument => 400,
        TonicCode::DeadlineExceeded => 504,
        TonicCode::NotFound => 404,
        TonicCode::AlreadyExists => 409,
        TonicCode::PermissionDenied => 403,
        TonicCode::ResourceExhausted => 429,
        TonicCode::FailedPrecondition => 400,
        TonicCode::Aborted => 409,
        TonicCode::OutOfRange => 400,
        TonicCode::Unimplemented => 501,
        TonicCode::Internal => 500,
        TonicCode::Unavailable => 503,
        TonicCode::DataLoss => 500,
        TonicCode::Unauthenticated => 401,
    }
}

//OLDER Err implementation . We may  pick some bones here

/*
impl fmt::Display for PVError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl Error for PVError { } //would we wan to implement source here?

#[derive(Debug)]
pub enum FRError {
    ApiError(PVError),
    HttpError(reqwest::Error),
    JsonError(serde_json::Error),
    DBError(Box<dyn std::error::Error + Send + Sync>), //is this ridiculous?
}

impl fmt::Display for FRError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FRError::ApiError(e) => write!(f, "{}", e),
            FRError::HttpError(e) => write!(f,"{}",e ),
            FRError::JsonError(e) => write!(f,"{}",e ),
            FRError::DBError(e) => write!(f,"{}",  e ),
            //FRError::DBError(e) => write!(f,"{}", e ),

        }
    }
}

impl Error for FRError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
       match self {
           FRError::ApiError(e) => Some(e),
           FRError::HttpError(e) => Some(e),
           FRError::JsonError(e) => Some(e),
           FRError::DBError(e) => Some(&***Box::new(e)), //this seems insane
//           FRError::DBError(e) => Some(e),
       }
    }
}


//so we can use ? directly. the ? will convert using into
impl From<PVError> for FRError {
    fn from(e: PVError) -> Self {
       FRError::ApiError(e)
    }
}

impl From<reqwest::Error> for FRError {
    fn from(e: reqwest::Error) -> Self {
        FRError::HttpError(e)
    }
}

impl From<serde_json::Error> for FRError {
    fn from(e: serde_json::Error) -> Self {
        FRError::JsonError(e)
    }
}
*/
/*
impl From<sqlx::error::Error> for FRError {
    fn from(e: sqlx::error::Error) -> Self {
        FRError::DBError(e)
    }
}
*/
