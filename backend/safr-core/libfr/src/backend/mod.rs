pub mod paravision;
mod pvtypes;
use crate::repo::EnrollmentMetadataRecord;
use crate::{
    AddFaceResult, DeleteFaceResult, EnrollData, EnrollmentDeleteResult, EnrollmentRosterItem,
    FRIdentity, FRResult, Face, GetFaceInfoResult, IDPair, Liveness, PossibleMatch,
    ResetEnrollmentsBackendResult, Template,
};
use bytes::Bytes;
use serde_json::Value;

#[allow(async_fn_in_trait)]
pub trait FRBackend: Send + Sync {
    //async fn enroll_face(&self, b64: String, details: EnrollDetails) -> FRResult;
    async fn create_enrollment(
        &self,
        enroll_data: &EnrollData,
        config: MatchConfig,
        ext_id: &str,
    ) -> FRResult<IDPair>; //create an enrollment for a single face
    async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult>; //delete an enrollment for a singel face
    async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord>;
    async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>>; //get a list of all the enrollments, or a subset for paging
    async fn reset_enrollments(&self) -> FRResult<ResetEnrollmentsBackendResult>; //delete the whole damn thing. away with you.
    async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>>;
    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>>;

    async fn validate_image(&self, image: Bytes) -> FRResult<Vec<Face>>;
    async fn generate_template(&self, image: Bytes) -> FRResult<Vec<Template>>;
    async fn liveness_check(&self, image: Bytes) -> FRResult<Vec<Face>>;
    async fn quality_check(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<Face>>;
    //async fn duplicate_check(&self, image: Bytes, config: MatchConfig) -> FRResult<PossibleMatch>;
    async fn create_identity(&self, template: Template, ext_id: &str) -> FRResult<IDSet>;

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<AddFaceResult>;
    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<DeleteFaceResult>;
    async fn get_face_info(&self, fr_id: &str) -> FRResult<GetFaceInfoResult>;
    async fn get_enrollments_by_last_name(&self, name: &str)
        -> FRResult<Vec<EnrollmentRosterItem>>;
    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()>;
}

pub struct IDSet {
    pub ext_id: String,
    pub fr_id: String,
}
#[derive(Copy, Clone)]
pub struct MatchConfig {
    pub min_match: f32,
    pub top_n: i32,
    pub min_dupe_match: f32,
    pub top_n_min_match: f32,
    pub min_quality: f32,
    pub min_acceptability: f32,
}
