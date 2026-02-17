pub mod backend;
pub mod remote;
pub mod v2;
use bytes::Bytes;
use libpv::errors::PVApiError;
use libtpass::errors::TPassError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::error::Error as SqlxError;
use thiserror::Error;

pub type FRResult<T> = Result<T, FRError>;

#[derive(Serialize, Deserialize, Debug, Error)]
#[error("{message}")]
pub struct FRError {
    pub code: u16,
    pub message: String,
    pub details: Option<Value>,
}

impl FRError {
    pub fn new() -> Self {
        Self {
            code: 500,
            message: "could not perform fr operation. this is a catch all.".to_string(),
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

    pub fn with_details(code: u16, message: &str, details: Value) -> Self {
        Self {
            code,
            message: message.to_string(),
            details: Some(details),
        }
    }
}

impl Default for FRError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<PVApiError> for FRError {
    fn from(pv: PVApiError) -> Self {
        Self {
            code: pv.code,
            message: pv.message,
            details: None,
        }
    }
}

impl From<&PVApiError> for FRError {
    fn from(pv: &PVApiError) -> Self {
        Self {
            code: pv.code,
            message: pv.message.clone(),
            details: None,
        }
    }
}

impl From<TPassError> for FRError {
    fn from(e: TPassError) -> Self {
        Self {
            code: 2000,
            message: e.to_string(),
            details: None,
        }
    }
}
impl From<SqlxError> for FRError {
    fn from(se: SqlxError) -> Self {
        Self {
            code: 1000, //don't know
            message: se.to_string(),
            details: None,
        }
    }
}

impl From<serde_json::Error> for FRError {
    fn from(se: serde_json::Error) -> Self {
        Self {
            code: 3000,
            message: se.to_string(),
            details: None,
        }
    }
}

//image and details are sent in a request using multipart formdata which we parse
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollData {
    pub image: Option<Bytes>,
    pub details: Option<EnrollDetails>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")] //i like kind more than type. type gets in the way.
pub enum EnrollDetails {
    Min {
        first_name: String,
        last_name: String,
    }, //only a name and local only
    TPass(Value), //TODO: this will be what NewProfileRequest contains, the tpass minimum.
}

//internal image transport is binary-only
#[derive(Debug)]
pub enum Image {
    Binary(Bytes),
}

#[derive(Debug)]
pub enum IDKind {
    String(String),
    Num(u64),
}
#[derive(Debug)]
pub enum SearchBy {
    //Name { first_name: String, last_name: String },
    Name {
        first_name: String,
        last_name: String,
    },
    //Partial(SearchRequest),
    ExtID(IDKind),
    ExtIDS(Vec<IDKind>),
}

//recognition types

#[derive(Debug, Serialize, Deserialize)]
pub struct PossibleMatch {
    pub fr_id: String,
    pub confidence: f32,
    pub ext_id: String,
    pub details: Option<Value>,
}

impl PossibleMatch {
    pub fn new(fr_id: String, confidence: f32) -> Self {
        Self {
            fr_id,
            confidence,
            ext_id: String::new(),
            details: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinDetails {
    pub fr_id: String,
    pub ext_id: String,
    pub details: Value,
}
///A combination of a set of attribute for a givent face and
///a possible list of matches from most likely to least likely
#[derive(Debug, Serialize, Deserialize)]
pub struct FRIdentity {
    pub face: Face,
    pub possible_matches: Vec<PossibleMatch>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoundingBox {
    pub origin: Point,
    pub width: f32,
    pub height: f32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Face {
    pub bbox: Option<BoundingBox>,
    pub acceptability: Option<f32>,
    pub quality: Option<f32>,
    pub mask: Option<f32>,
    pub liveness: Option<Liveness>,
    //pub extra: Option<Stuff>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Liveness {
    pub is_live: bool,
    pub feedback: Vec<String>,
    pub score: f32,
}

pub mod utils {

    pub fn round(x: f64, decimals: u32) -> f64 {
        let y = 10i32.pow(decimals) as f64;
        (x * y).round() / y
    }

    pub fn roundf32(x: f32, decimals: u32) -> f32 {
        let y = 10i32.pow(decimals) as f32;
        (x * y).round() / y
    }
}
