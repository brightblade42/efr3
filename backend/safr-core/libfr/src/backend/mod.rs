pub mod paravision;
pub mod paravision_grpc;
mod pvtypes;
use crate::{EnrollData, FRIdentity, FRResult};
use bytes::Bytes;
use serde_json::Value;

#[allow(async_fn_in_trait)]
pub trait FRBackend: Send + Sync {
    //async fn enroll_face(&self, b64: String, details: EnrollDetails) -> FRResult;
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<String>,
    ) -> FRResult<Value>; //create an enrollment for a single face
    async fn delete_enrollment(&self, fr_id: &str) -> FRResult<Value>; //delete an enrollment for a singel face
    async fn get_enrollment_metadata(&self) -> FRResult<Value>;
    async fn get_enrollment_roster(&self) -> FRResult<Value>; //get a list of all the enrollments, or a subset for paging
    async fn reset_enrollments(&self) -> FRResult<Value>; //delete the whole damn thing. away with you.
    async fn detect_face(&self, image: Bytes, spoof_check: bool) -> FRResult<Value>;
    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>>;

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<Value>;
    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value>;
    async fn get_face_info(&self, fr_id: &str) -> FRResult<Value>;
    async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>>;
    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()>;
}

#[derive(Copy, Clone)]
pub struct MatchConfig {
    pub min_match: f32,
    pub top_n: i32,
    pub min_dupe_match: f32,
    pub top_n_min_match: f32,
}
