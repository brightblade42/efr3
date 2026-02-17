use std::sync::Arc;

use bytes::Bytes;
use libfr::{
    backend::{paravision::PVBackend, paravision_grpc::PVGrpcBackend, FRBackend, MatchConfig},
    remote::{RegistrationPair, Remote, SearchResult},
    EnrollData, FRIdentity, FRResult, SearchBy,
};
use libtpass::api::TPassClient;
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
    ParavisionGrpc(PVGrpcBackend),
    Paravision(PVBackend),
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
            "paravision-grpc" | "pv-grpc" => Ok(Self::ParavisionGrpc(PVGrpcBackend::new(
                proc_url, ident_url, db,
            ))),
            "paravision" | "pv" => Ok(Self::Paravision(PVBackend::new(proc_url, ident_url, db))),
            _ => Err(format!(
                "unsupported FR_BACKEND '{}'; supported values: paravision-grpc, paravision",
                raw
            )),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ParavisionGrpc(_) => "paravision-grpc",
            Self::Paravision(_) => "paravision",
        }
    }
}

impl FRBackend for FREngine {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<u64>,
    ) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => {
                backend.create_enrollment(enroll_data, config, ext_id).await
            }
            Self::Paravision(backend) => {
                backend.create_enrollment(enroll_data, config, ext_id).await
            }
        }
    }

    async fn delete_enrollment(&self, fr_id: &str) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.delete_enrollment(fr_id).await,
            Self::Paravision(backend) => backend.delete_enrollment(fr_id).await,
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.get_enrollment_metadata().await,
            Self::Paravision(backend) => backend.get_enrollment_metadata().await,
        }
    }

    async fn get_enrollment_roster(&self) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.get_enrollment_roster().await,
            Self::Paravision(backend) => backend.get_enrollment_roster().await,
        }
    }

    async fn reset_enrollments(&self) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.reset_enrollments().await,
            Self::Paravision(backend) => backend.reset_enrollments().await,
        }
    }

    async fn detect_face(&self, image: Bytes, spoof_check: bool) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.detect_face(image.clone(), spoof_check).await,
            Self::Paravision(backend) => backend.detect_face(image, spoof_check).await,
        }
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        match self {
            Self::ParavisionGrpc(backend) => backend.recognize(image.clone(), config).await,
            Self::Paravision(backend) => backend.recognize(image, config).await,
        }
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.add_face(fr_id, image.clone()).await,
            Self::Paravision(backend) => backend.add_face(fr_id, image).await,
        }
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.delete_face(fr_id, face_id).await,
            Self::Paravision(backend) => backend.delete_face(fr_id, face_id).await,
        }
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<Value> {
        match self {
            Self::ParavisionGrpc(backend) => backend.get_face_info(fr_id).await,
            Self::Paravision(backend) => backend.get_face_info(fr_id).await,
        }
    }

    async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
        match self {
            Self::ParavisionGrpc(backend) => backend.get_enrollments_by_last_name(name).await,
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
            Self::ParavisionGrpc(backend) => {
                backend.log_identity(fr_identity, extra, location).await
            }
            Self::Paravision(backend) => backend.log_identity(fr_identity, extra, location).await,
        }
    }
}
