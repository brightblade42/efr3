pub mod backend;
pub mod remote;
pub mod repo;
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
        Self { code, message: message.to_string(), details: None }
    }

    pub fn with_details(code: u16, message: &str, details: Value) -> Self {
        Self { code, message: message.to_string(), details: Some(details) }
    }
}

impl Default for FRError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<PVApiError> for FRError {
    fn from(pv: PVApiError) -> Self {
        Self { code: pv.code, message: pv.message, details: None }
    }
}

impl From<&PVApiError> for FRError {
    fn from(pv: &PVApiError) -> Self {
        Self { code: pv.code, message: pv.message.clone(), details: None }
    }
}

impl From<TPassError> for FRError {
    fn from(e: TPassError) -> Self {
        Self { code: 2000, message: e.to_string(), details: None }
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
        Self { code: 3000, message: se.to_string(), details: None }
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
    Min { first_name: String, last_name: String, ext_id: Option<String> }, //only a name and local only
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
    Name { first_name: String, last_name: String },
    //Partial(SearchRequest),
    ExtID(IDKind),
    ExtIDS(Vec<IDKind>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrollmentCreateResult {
    pub fr_id: String,
    pub ext_id: u64,
    pub ext_id_str: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrollmentDeleteResult {
    pub fr_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrollmentRosterItem {
    pub fr_id: Option<String>,
    pub ext_id: u64,
    pub ext_id_str: String,
    pub details: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetEnrollmentsBackendResult {
    pub msg: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetEnrollmentsResult {
    pub msg: String,
    pub local_reset: repo::EnrollmentResetRecord,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrollmentFaceInfo {
    pub id: String,
    pub identity_id: String,
    pub created_at: String,
    pub model: String,
    pub quality: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddFaceResult {
    pub faces: Vec<EnrollmentFaceInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteFaceResult {
    pub rows_affected: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetFaceInfoResult {
    pub faces: Vec<EnrollmentFaceInfo>,
    pub next_page_token: String,
    pub total_size: i32,
}

//recognition types

#[derive(Debug, Serialize, Deserialize)]
pub struct PossibleMatch {
    pub fr_id: String,
    #[serde(alias = "confidence")]
    pub score: f32,
    #[serde(default, alias = "confidence_pct")]
    pub score_pct: f32,
    pub ext_id: String,
    pub details: Option<Value>,
}

impl PossibleMatch {
    pub fn new(fr_id: String, score: f32) -> Self {
        Self {
            fr_id,
            score,
            score_pct: utils::score_to_percentage(score),
            ext_id: String::new(),
            details: None,
        }
    }

    pub fn refresh_score_percentage(&mut self) {
        self.score_pct = utils::score_to_percentage(self.score);
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

    pub fn score_to_percentage(score: f32) -> f32 {
        roundf32(score * 100.0, 2)
    }

    pub fn normalize_score_threshold(threshold: f32) -> f32 {
        let raw = if threshold > 1.0 { threshold / 100.0 } else { threshold };

        raw.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{utils, PossibleMatch};
    use serde_json::json;

    #[test]
    fn possible_match_serializes_score_field() {
        let possible_match = PossibleMatch {
            fr_id: "i_test".to_string(),
            score: 0.99,
            score_pct: 99.0,
            ext_id: "123".to_string(),
            details: None,
        };

        let value = serde_json::to_value(possible_match).expect("serialize possible match");
        assert!(value.get("score").is_some());
        assert_eq!(value.get("score_pct").and_then(|value| value.as_f64()), Some(99.0));
        assert!(value.get("confidence").is_none());
    }

    #[test]
    fn possible_match_deserializes_confidence_alias() {
        let value = json!({
            "fr_id": "i_test",
            "confidence": 0.75,
            "ext_id": "456",
            "details": null
        });

        let mut possible_match: PossibleMatch =
            serde_json::from_value(value).expect("deserialize possible match from confidence");
        assert!((possible_match.score - 0.75).abs() < f32::EPSILON);
        possible_match.refresh_score_percentage();
        assert_eq!(possible_match.score_pct, 75.0);
    }

    #[test]
    fn score_to_percentage_is_rounded() {
        assert_eq!(utils::score_to_percentage(0.98765), 98.77);
    }

    #[test]
    fn normalize_score_threshold_accepts_ratio_and_percent() {
        assert_eq!(utils::normalize_score_threshold(0.98), 0.98);
        assert_eq!(utils::normalize_score_threshold(98.0), 0.98);
    }
}
