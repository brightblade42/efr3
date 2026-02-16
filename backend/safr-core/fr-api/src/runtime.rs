use std::sync::Arc;

use libfr::{
    backend::{paravision::PVBackend, FRBackend, MatchConfig},
    remote::{RegistrationPair, Remote, SearchResult},
    EnrollData, FRIdentity, FRResult, SearchBy,
};
use libtpass::api::TPassClient;
use serde_json::Value;
use sqlx::PgPool;

const DEFAULT_BACKEND: &str = "paravision";
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
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<Value> {
        match self {
            Self::TPass(client) => client.register_enrollment(reg_pair).await,
        }
    }

    async fn unregister_enrollment(&self) -> FRResult<Value> {
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

    async fn search_many(&self, search: SearchBy, include_img: bool) -> FRResult<Vec<Value>> {
        match self {
            Self::TPass(client) => client.search_many(search, include_img).await,
        }
    }
}

#[derive(Clone)]
pub enum FREngine {
    Paravision(PVBackend<RemoteRuntime>),
}

impl FREngine {
    pub fn from_env(
        backend: Option<String>,
        proc_url: String,
        ident_url: String,
        db: PgPool,
        remote: RemoteRuntime,
    ) -> Result<Self, String> {
        let raw = backend.unwrap_or_else(|| DEFAULT_BACKEND.to_string());
        match raw.to_ascii_lowercase().as_str() {
            "paravision" | "pv" => Ok(Self::Paravision(PVBackend::new(
                proc_url, ident_url, db, remote,
            ))),
            _ => Err(format!(
                "unsupported FR_BACKEND '{}'; supported values: paravision",
                raw
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Paravision(_) => "paravision",
        }
    }
}

impl FRBackend for FREngine {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
    ) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.create_enrollment(enroll_data, config).await,
        }
    }

    async fn delete_enrollment(&self, fr_id: &str) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.delete_enrollment(fr_id).await,
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.get_enrollment_metadata().await,
        }
    }

    async fn get_enrollment_roster(&self) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.get_enrollment_roster().await,
        }
    }

    async fn reset_enrollments(&self) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.reset_enrollments().await,
        }
    }

    async fn detect_face(&self, b64: String, spoof_check: bool) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.detect_face(b64, spoof_check).await,
        }
    }

    async fn recognize(&self, b64: String, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        match self {
            Self::Paravision(backend) => backend.recognize(b64, config).await,
        }
    }

    async fn add_face(&self, fr_id: &str, b64: String) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.add_face(fr_id, b64).await,
        }
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.delete_face(fr_id, face_id).await,
        }
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<Value> {
        match self {
            Self::Paravision(backend) => backend.get_face_info(fr_id).await,
        }
    }

    async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
        match self {
            Self::Paravision(backend) => backend.get_enrollments_by_last_name(name).await,
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
        }
    }
}
