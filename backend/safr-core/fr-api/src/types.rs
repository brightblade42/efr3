use libfr::EnrollData;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

// ============= V1 backport shenanigans ============================

//TPASS sends an older structure for enrollment.
#[derive(Serialize, Deserialize, Debug)]
pub struct TPassCandidate {
    pub ccode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_or_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comp_id: Option<String>,
}

//NOTE: candidates is a vec but pretty sure tpass only sends one at a time.
#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollCommand {
    pub command: String,
    pub candidates: Vec<TPassCandidate>,
}

impl From<EnrollCommand> for EnrollData {
    fn from(_value: EnrollCommand) -> Self {
        todo!()
    }
}

impl From<&EnrollCommand> for EnrollData {
    fn from(_value: &EnrollCommand) -> Self {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollmentResultV1 {
    pub dupe_count: u32,
    pub duplicates: Vec<DupeItem>,
    pub enroll_count: u32,
    pub no_img_count: u32,
    pub rec_fail_count: u32,
    pub search_count: u32,
}

impl Default for EnrollmentResultV1 {
    fn default() -> Self {
        Self {
            dupe_count: 0,
            duplicates: vec![],
            enroll_count: 1,
            no_img_count: 0,
            rec_fail_count: 0,
            search_count: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DupeItem {
    pub ccode: u64,
    pub identities: Vec<Value>,
}

impl Default for DupeItem {
    fn default() -> Self {
        Self {
            ccode: 0,
            identities: vec![json!({
                "id": "123abc456def",
                "created_at": "2023-01-01T01:01:00", //these aren't useful.
                "updated_at": "2023-01-01T01:01:00", //dummy vals
                "confidence": 0.90
            })],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteEnrollmentsRequestV1 {
    pub fr_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_delete: Option<bool>, //includes requesting delete to linked servers like tpass
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFaceResponseV1 {
    pub face_id: String,
    pub fr_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetFacesRequest {
    pub fr_id: String,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetFacesResponse {
    pub faces: Vec<FaceInfo>,
    pub next_page_token: String,
    pub total_size: i32,
}
//NOTE The following types are still in libpv. once v1 is killed,
//this will all cleanup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaceInfo {
    pub id: String,
    pub created_at: String,
    pub model: String,
    pub quality: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddFaceResponse {
    pub faces: Vec<FaceInfo>,
}

#[derive(Debug, Error)]
pub enum AddFaceError {
    #[error("add-face response had no faces")]
    NoFaces,
}

impl TryFrom<AddFaceResponse> for AddFaceResponseV1 {
    type Error = AddFaceError;

    fn try_from(resp: AddFaceResponse) -> Result<Self, Self::Error> {
        let face = resp.faces.into_iter().next().ok_or(AddFaceError::NoFaces)?;
        Ok(Self {
            face_id: face.id,
            fr_id: "".to_string(),
        })
    }
}
