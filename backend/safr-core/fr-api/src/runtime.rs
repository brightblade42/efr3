use std::sync::Arc;

use bytes::Bytes;
#[cfg(test)]
use libfr;
use libfr::{
    backend::{paravision::PVBackend, FRBackend, IDSet, MatchConfig},
    remote::{RegistrationPair, Remote, SearchResult},
    repo::EnrollmentMetadataRecord,
    DeleteFaceResult, EnrollData, EnrolledFaceInfo, EnrollmentRosterItem, FRIdentity, FRResult,
    Face, IDPair, SearchBy, Template,
};
use libtpass::api::TPassClient;
#[cfg(test)]
use serde_json::json;
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
            _ => Err(format!("unsupported FR_REMOTE '{}'; supported values: tpass", raw)),
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

    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_by_ids(search, include_img).await,
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
        face: &Face,
        config: MatchConfig,
        ext_id: &str,
    ) -> FRResult<IDPair> {
        match self {
            Self::Paravision(backend) => backend.create_enrollment(face, config, ext_id).await,
            #[cfg(test)]
            Self::Mock => Ok(IDPair { fr_id: "mock-fr-id".to_string(), ext_id: "123".to_string() }),
        }
    }

    //TODO: indicate if we only want most prominent? or do after the fact?
    async fn generate_template(&self, image: Bytes) -> FRResult<Vec<libfr::Template>> {
        match self {
            Self::Paravision(backend) => backend.generate_template(image).await,
            #[cfg(test)]
            Self::Mock => Ok(vec![]),
        }
    }

    async fn create_identity(&self, template: Template, ext_id: &str) -> FRResult<IDSet> {
        match self {
            Self::Paravision(backend) => backend.create_identity(template, ext_id).await,
            #[cfg(test)]
            Self::Mock => Ok(IDSet { ext_id: "112233".to_string(), fr_id: "abc_123".to_string() }),
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
                ext_id: "123".to_string(),
                details: json!({"first_name":"Test","last_name":"User"}),
            }]),
        }
    }

    async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        match self {
            Self::Paravision(backend) => backend.detect_faces(image, liveness_check).await,
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

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<EnrolledFaceInfo> {
        match self {
            Self::Paravision(backend) => backend.add_face(fr_id, image).await,
            #[cfg(test)]
            Self::Mock => Ok(EnrolledFaceInfo {
                face_id: "mock-face-id".to_string(),
                fr_id: fr_id.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                quality: 0.99,
            }),
        }
    }

    async fn delete_faces(&self, fr_id: &str, face_ids: Vec<String>) -> FRResult<DeleteFaceResult> {
        match self {
            Self::Paravision(backend) => backend.delete_faces(fr_id, face_ids).await,
            #[cfg(test)]
            Self::Mock => {
                let _ = (fr_id, face_ids);
                Ok(DeleteFaceResult { rows_affected: 1 })
            }
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
}
