use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use reqwest::Error;
use serde_json::Value;

use serde::{Deserialize, Serialize};
//----- A better error strategy
//top level, our domain specific errors will be transformed to AppErrors for retunning to users
use libfr::errors::FRError;
use libtpass::errors::TPassError;

#[derive(Debug)]
pub enum AppError {
    Generic(String), //for very simple messages or an error we're not sure how to format yet.
    Standard(StandardError), //standard json error message. the http call was good but there was an api failure.
}

impl From<TPassError> for AppError {
    fn from(te: TPassError) -> Self {
        AppError::Standard(StandardError { code: 5000, message: te.to_string(), details: None })
    }
}

//convert from library errors to App level errors.
impl From<FRError> for AppError {
    fn from(fe: FRError) -> Self {
        AppError::Standard(StandardError {
            code: fe.code,
            message: fe.message,
            details: fe.details,
        })
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
                let std_err = StandardError { code: 0, message: msg, details: None };
                (StatusCode::OK, Json(std_err))
            }
        };

        (status, error).into_response()
    }
}

//a general format for returning most api based errors to client.
#[derive(Serialize, Deserialize, Debug)]
pub struct StandardError {
    pub code: u16,
    pub message: String,
    pub details: Option<Value>,
}
