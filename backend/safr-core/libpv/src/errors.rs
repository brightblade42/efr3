//use std::any::TypeId;
//use std::backtrace::Backtrace;
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

//TODO: not using serde here are we?
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

//TODO: figure out if converting tonic to http code actually makes sense
// i think it does since our client is thinking in http terms..not sure thou
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
