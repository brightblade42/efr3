use serde::{Deserialize, Serialize};
//use sqlx::types::{Json, chrono::NaiveTime};

//TODO: rename to something like PagedIdentities
#[derive(Serialize, Deserialize, Debug)]
pub struct Identities {
    pub identities: Vec<Identity>,
    pub next_page_token: String,
    pub total_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    pub id: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub updated_at: String,
    pub group_ids: Option<Vec<String>>,
}

///A typed set of Request parameter for the pv process_full_image api call
#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessFullImageRequest {
    pub image: String,
    pub outputs: Option<Vec<String>>,
    pub find_most_prominent_face: bool,
}

impl ProcessFullImageRequest {
    pub fn new(image: String, outputs: Option<Vec<String>>, prominent_faces: bool) -> Self {
        Self {
            image,
            outputs,
            find_most_prominent_face: prominent_faces,
        }
    }
}

impl Default for ProcessFullImageRequest {
    fn default() -> Self {
        Self {
            image: "".to_string(),
            outputs: Some(vec![
                String::from("BOUNDING_BOX"),
                String::from("EMBEDDING"),
                //String::from("ALIGNED_FACE_IMAGE"),
                String::from("QUALITY"),
                String::from("MASK"),
            ]),
            find_most_prominent_face: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetFacesInput {
    pub fr_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetFacesResponse {
    pub faces: Vec<FaceInfo>,
    pub next_page_token: String,
    pub total_size: i32,
}

//TODO: I think this is supposed to be Response not request, maybe not.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetIdentitiesInput {
    pub page_size: u32,
    pub page_token: Option<String>,
    pub group_ids: Option<Vec<String>>,
}

//returned from Process image api call
//#[derive(Serialize, Deserialize, Debug,Clone, sqlx::FromRow)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessImageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faces: Option<Vec<Face>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_prominent_face_idx: Option<i32>,
}

//==== Adding / Removing secondary faces

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddFaceInput {
    pub identity_id: String, //the main enrollment id
    pub embeddings: Vec<Embedding>,
    pub threshold: f32,
    pub qualities: Vec<f32>, //TODO: not sure we're going to use this. is ther a default
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FaceInfo {
    pub id: String,
    pub identity_id: String,
    pub created_at: String,
    pub model: String,
    pub quality: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddFaceResponse {
    pub faces: Vec<FaceInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteFaceInput {
    pub fr_id: String, //This was an integer in older paravision apis
    pub face_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteFaceResponse {
    pub rows_affected: i32,
}

//---- end of Secondary Face types

#[derive(Serialize, Deserialize, Debug)]
pub struct LookupInput {
    pub embeddings: Vec<Embedding>,
    #[serde(skip_serializing)]
    pub faces: Option<Vec<Face>>,
    pub limit: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityMatch {
    pub identity: Identity,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LookupIdentity {
    pub matches: Vec<IdentityMatch>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LookupResponse {
    pub face: Face,
    pub identities: LookupIdentities,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LookupIdentities {
    pub lookup_identities: Vec<LookupIdentity>,
}

//Not sure how to do this...

// #[derive(Serialize, Deserialize, Debug)]
// pub struct DeleteIdentitiesResponse {
//     pub rows_affected: u32,
//     pub ids: Option<Vec<String>>
// }

// impl DeleteIdentitiesResponse {
//     //if there's only a single result, extract it.
//     pub fn single(&self) -> Option<String> {
//         if self.rows_affected == 1 {
//             self.ids.as_ref().and_then(|i| i.iter().nth(0).map(|s| s.to_string()))
//         } else {
//             None
//         }
//     }
// }

//#[derive(Serialize, Deserialize, Debug)]
//pub struct DeletedIdentity(String);

#[derive(Serialize, Deserialize, Debug)]
pub struct IDTrips {
    pub id: i32,
    pub fr_id: String,
    pub ccode: String,
}

//when registering enrollments with outside sources

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrationError {
    #[serde(rename = "cCode")]
    c_code: u64,
    id: String,
    error: bool,
    #[serde(rename = "errMessage")]
    err_message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateIdentitiesInput {
    pub embeddings: Vec<Embedding>,
    pub threshold: f32, //NOTE: threshold for what?
    pub qualities: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ids: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteIdentitiesInput {
    pub ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ids: Option<Vec<String>>,
}

impl From<&str> for DeleteIdentitiesInput {
    fn from(id: &str) -> Self {
        Self {
            ids: vec![id.to_string()],
            external_ids: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteIdentityResponse {
    fr_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateIdentitiesResponse {
    pub identities: Vec<Identity>,
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
pub struct Landmarks {
    pub eye_left: Point,
    pub eye_right: Point,
    pub nose: Point,
    pub mouth_left: Point,
    pub mouth_right: Point,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Distribution {
    confidence: f64,
    value: String,
}

//things like gender, age
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonalAttribute {
    pub confidence: f64,
    pub value: String,
    pub distribution: Vec<Distribution>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Embedding {
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Validness {
    pub is_valid: bool,
    #[serde(default)]
    pub feedback: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Liveness {
    pub liveness_probability: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Face {
    pub bounding_box: Option<BoundingBox>,
    pub landmarks: Option<Landmarks>,
    pub embedding: Option<Vec<f32>>, //TODO: should this be Embedding? i think we just convert later.
    pub ages: Option<PersonalAttribute>,
    pub genders: Option<PersonalAttribute>,
    pub aligned_face_image: Option<String>, //cropped face from a larger image of possibly many faces
    pub acceptability: Option<f32>,
    pub quality: Option<f32>,
    pub mask: Option<f32>,
    pub liveness_validness: Option<Validness>,
    pub liveness: Option<Liveness>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthCheckResponse {
    pub status: String,
}
