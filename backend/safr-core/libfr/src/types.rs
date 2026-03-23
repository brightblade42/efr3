//! Core domain and transport types shared across the SAFR backend.
//!
//! These types intentionally span several boundaries:
//! - API input normalized by `fr-api`
//! - workflow input/output used by [`crate::service::FRService`]
//! - backend and remote integration payloads translated into stable internal models

use bytes::Bytes;

use crate::errors::FRError;
use crate::pv::types::timestamp_to_rfc3339;
use crate::tpass::types::TPassProfile;
use crate::utils;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub type FRResult<T> = Result<T, FRError>;

/// Remote-profile details attached to a recognition match or search result.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "remote", content = "profile")]
pub enum RemoteDetails {
    TPass(TPassProfile),
}

impl RemoteDetails {
    pub fn as_tpass(&self) -> Option<&TPassProfile> {
        match self {
            Self::TPass(profile) => Some(profile),
        }
    }

    pub fn into_tpass(self) -> Option<TPassProfile> {
        match self {
            Self::TPass(profile) => Some(profile),
        }
    }

    pub fn ext_id(&self) -> Option<String> {
        match self {
            Self::TPass(profile) => profile.ccode.map(|id| id.to_string()),
        }
    }

    pub fn first_name(&self) -> Option<&str> {
        match self {
            Self::TPass(profile) => profile.f_name.as_deref(),
        }
    }

    pub fn last_name(&self) -> Option<&str> {
        match self {
            Self::TPass(profile) => profile.l_name.as_deref(),
        }
    }

    pub fn img_url(&self) -> Option<&str> {
        match self {
            Self::TPass(profile) => profile.img_url.as_deref(),
        }
    }
}

/// Options that control match enrichment in recognition responses.
#[derive(Serialize, Deserialize, Debug)]
pub struct RecognizeOpts {
    pub include_details: bool,
}

/// Enrollment input normalized from multipart form data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollData {
    pub image: Option<Bytes>,
    pub details: Option<EnrollDetails>,
}

/// Supported enrollment detail sources.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")] //i like kind more than type. type gets in the way.
pub enum EnrollDetails {
    Min { first_name: String, last_name: String, ext_id: Option<String> }, //only a name and local only
    TPass(TPassProfile), //TODO: this will be what NewProfileRequest contains, the tpass minimum.
}

//internal image transport is binary-only
// #[derive(Debug)]
// pub enum Image {
//     Binary(Bytes),
// }

/// Lookup strategies supported by remote search operations.
#[derive(Debug)]
pub enum SearchBy {
    //Name { first_name: String, last_name: String },
    Name { first_name: String, last_name: String },
    //Partial(SearchRequest),
    ExtID(String),
    ExtIDS(Vec<String>),
}

/// Search result returned from the remote system.
#[derive(Debug)]
pub struct SearchResult {
    pub image: Option<Image>,
    pub id: Option<String>,
    pub details: Option<RemoteDetails>,
}

/// Mapping between an FR identity id and an external person identifier.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IDPair {
    pub fr_id: String,
    pub ext_id: String,
}

impl IDPair {
    pub fn new(fr_id: String, ext_id: String) -> Self {
        Self { fr_id, ext_id }
    }
}

/// Result returned after a successful enrollment delete request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrollmentDeleteResult {
    pub fr_id: String,
}

/// Metadata about one stored face belonging to an identity.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnrolledFaceInfo {
    pub face_id: String,
    pub fr_id: String,
    pub created_at: String,
    pub quality: f32,
}

impl From<crate::pv::pv_grpc::identity_grpc::identity::Face> for EnrolledFaceInfo {
    fn from(f: crate::pv::pv_grpc::identity_grpc::identity::Face) -> Self {
        Self {
            face_id: f.id,
            fr_id: f.identity_id,
            quality: f.quality,
            created_at: timestamp_to_rfc3339(f.created_at),
        }
    }
}

/// Result returned after deleting one or more stored faces.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteFaceResult {
    pub rows_affected: i32,
}

/// One possible identity match returned by the FR backend.
#[derive(Debug, Serialize, Deserialize)]
pub struct PossibleMatch {
    pub fr_id: String,
    #[serde(alias = "confidence")]
    pub score: f32,
    #[serde(default, alias = "confidence_pct")]
    pub score_pct: f32,
    pub ext_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>, //most likely some kind of remote profile info
}

impl PossibleMatch {
    /// Construct a match with an automatically derived percentage field.
    pub fn new(fr_id: String, score: f32) -> Self {
        Self {
            fr_id,
            score,
            score_pct: utils::score_to_percentage(score),
            ext_id: String::new(),
            details: None,
        }
    }

    /// Recompute the cached percentage representation after changing `score`.
    pub fn refresh_score_percentage(&mut self) {
        self.score_pct = utils::score_to_percentage(self.score);
    }
}

/// Minimal details payload used by some legacy API compatibility flows.
#[derive(Debug, Serialize, Deserialize)]
pub struct MinDetails {
    pub fr_id: String,
    pub ext_id: String,
    pub details: Value,
}
/// One detected face plus its ranked candidate identity matches.
#[derive(Debug, Serialize, Deserialize)]
pub struct FRIdentity {
    pub face: Face,
    pub possible_matches: Vec<PossibleMatch>,
}

/// Cartesian point used for face bounding boxes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Bounding box describing the detected face position in an image.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoundingBox {
    pub origin: Point,
    pub width: f32,
    pub height: f32,
}
/// Face-level attributes returned from processor and recognition operations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Face {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
    pub acceptability: Option<f32>,
    pub quality: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liveness: Option<Liveness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    //pub extra: Option<Stuff>
}

/// Embedding template returned by an FR backend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Template {
    pub embedding: Vec<f32>,
}

/// Image payload used when remote lookups also include a person photo.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub url: Option<String>, //url or file path. should this be a path str?
    pub bytes: Option<Bytes>,
}

/// Liveness data returned when supported by the FR backend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Liveness {
    pub is_live: bool,
    pub feedback: Vec<String>,
    pub score: f32,
}

/// Recognition and enrollment thresholds used across the service layer.
#[derive(Copy, Clone)]
pub struct MatchConfig {
    pub min_match: f32,
    pub top_n: i32,
    pub min_dupe_match: f32,
    pub top_n_min_match: f32,
    pub min_quality: f32,
    pub min_acceptability: f32,
    pub include_details: bool,
}
