use std::sync::Arc;

use bytes::Bytes;
#[cfg(test)]
use libfr::EnrollmentFaceInfo;
use libfr::{
    backend::{paravision::PVBackend, FRBackend, MatchConfig},
    remote::{RegistrationPair, Remote, SearchManyResult, SearchResult},
    v2::domain::EnrollmentMetadataRecord,
    AddFaceResult, DeleteFaceResult, EnrollData, EnrollmentCreateResult, EnrollmentDeleteResult,
    EnrollmentRosterItem, FRIdentity, FRResult, Face, GetFaceInfoResult,
    ResetEnrollmentsBackendResult, SearchBy,
};
use libtpass::api::TPassClient;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use sqlx::PgPool;

const DEFAULT_BACKEND: &str = "paravision-grpc";
const DEFAULT_REMOTE: &str = "tpass";

#[derive(Clone)]
pub enum RemoteRuntime {
    TPass(Arc<TPassClient>),
}

impl RemoteRuntime {
    pub fn from_env(
        remote: Option<String>,
        tpass_client: Arc<TPassClient>,
    ) -> Result<Self, String> {
        let raw = remote.unwrap_or_else(|| DEFAULT_REMOTE.to_string());
        match raw.to_ascii_lowercase().as_str() {
            "tpass" => Ok(Self::TPass(tpass_client)),
            _ => Err(format!(
                "unsupported FR_REMOTE '{}'; supported values: tpass",
                raw
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::TPass(_) => "tpass",
        }
    }
}

impl Remote for RemoteRuntime {
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.register_enrollment(reg_pair).await,
        }
    }

    async fn unregister_enrollment(&self) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.unregister_enrollment().await,
        }
    }

    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search(enroll_data).await,
        }
    }

    async fn search_one(
        &self,
        search: SearchBy,
        include_image: bool,
    ) -> FRResult<Option<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_one(search, include_image).await,
        }
    }

    async fn search_many(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchManyResult>> {
        match self {
            Self::TPass(client) => client.search_many(search, include_img).await,
        }
    }
}

#[derive(Clone)]
pub enum FREngine {
    Paravision(PVBackend),
    #[cfg(test)]
    Mock,
}

impl FREngine {
    pub fn from_env(
        backend: Option<String>,
        proc_url: String,
        ident_url: String,
        db: PgPool,
    ) -> Result<Self, String> {
        let raw = backend.unwrap_or_else(|| DEFAULT_BACKEND.to_string());
        match raw.to_ascii_lowercase().as_str() {
            "paravision-grpc" | "pv-grpc" | "paravision" | "pv" => {
                Ok(Self::Paravision(PVBackend::new(proc_url, ident_url, db)))
            }
            _ => Err(format!(
                "unsupported FR_BACKEND '{}'; supported values: paravision-grpc, paravision",
                raw
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Paravision(_) => "paravision",
            #[cfg(test)]
            Self::Mock => "mock",
        }
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self::Mock
    }
}

impl FRBackend for FREngine {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<String>,
    ) -> FRResult<EnrollmentCreateResult> {
        match self {
            Self::Paravision(backend) => {
                backend.create_enrollment(enroll_data, config, ext_id).await
            }
            #[cfg(test)]
            Self::Mock => Ok(EnrollmentCreateResult {
                fr_id: "mock-fr-id".to_string(),
                ext_id: 123,
                ext_id_str: "123".to_string(),
            }),
        }
    }

    async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult> {
        match self {
            Self::Paravision(backend) => backend.delete_enrollment(fr_id).await,
            #[cfg(test)]
            Self::Mock => Ok(EnrollmentDeleteResult {
                fr_id: fr_id.to_string(),
            }),
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        match self {
            Self::Paravision(backend) => backend.get_enrollment_metadata().await,
            #[cfg(test)]
            Self::Mock => Ok(EnrollmentMetadataRecord {
                profiles_total: 1,
                profiles_with_fr_id: 1,
                images_total: 1,
                registration_errors_total: 0,
                enrollment_logs_total: 0,
            }),
        }
    }

    async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>> {
        match self {
            Self::Paravision(backend) => backend.get_enrollment_roster().await,
            #[cfg(test)]
            Self::Mock => Ok(vec![EnrollmentRosterItem {
                fr_id: Some("mock-fr-id".to_string()),
                ext_id: 123,
                ext_id_str: "123".to_string(),
                details: json!({"first_name":"Test","last_name":"User"}),
            }]),
        }
    }

    async fn reset_enrollments(&self) -> FRResult<ResetEnrollmentsBackendResult> {
        match self {
            Self::Paravision(backend) => backend.reset_enrollments().await,
            #[cfg(test)]
            Self::Mock => Ok(ResetEnrollmentsBackendResult {
                msg: "mock reset".to_string(),
            }),
        }
    }

    async fn detect_face(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        match self {
            Self::Paravision(backend) => backend.detect_face(image, liveness_check).await,
            #[cfg(test)]
            Self::Mock => Ok(Vec::new()),
        }
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        match self {
            Self::Paravision(backend) => backend.recognize(image, config).await,
            #[cfg(test)]
            Self::Mock => Ok(vec![]),
        }
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<AddFaceResult> {
        match self {
            Self::Paravision(backend) => backend.add_face(fr_id, image).await,
            #[cfg(test)]
            Self::Mock => Ok(AddFaceResult {
                faces: vec![EnrollmentFaceInfo {
                    id: "mock-face-id".to_string(),
                    identity_id: fr_id.to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    model: "mock".to_string(),
                    quality: 0.99,
                }],
            }),
        }
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<DeleteFaceResult> {
        match self {
            Self::Paravision(backend) => backend.delete_face(fr_id, face_id).await,
            #[cfg(test)]
            Self::Mock => {
                let _ = (fr_id, face_id);
                Ok(DeleteFaceResult { rows_affected: 1 })
            }
        }
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<GetFaceInfoResult> {
        match self {
            Self::Paravision(backend) => backend.get_face_info(fr_id).await,
            #[cfg(test)]
            Self::Mock => Ok(GetFaceInfoResult {
                faces: vec![EnrollmentFaceInfo {
                    id: "mock-face-id".to_string(),
                    identity_id: fr_id.to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    model: "mock".to_string(),
                    quality: 0.99,
                }],
                next_page_token: String::new(),
                total_size: 1,
            }),
        }
    }

    async fn get_enrollments_by_last_name(
        &self,
        name: &str,
    ) -> FRResult<Vec<EnrollmentRosterItem>> {
        match self {
            Self::Paravision(backend) => backend.get_enrollments_by_last_name(name).await,
            #[cfg(test)]
            Self::Mock => Ok(vec![]),
        }
    }

    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        match self {
            Self::Paravision(backend) => backend.log_identity(fr_identity, extra, location).await,
            #[cfg(test)]
            Self::Mock => Ok(()),
        }
    }
}
