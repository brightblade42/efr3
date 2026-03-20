use crate::backend::paravision::PVBackend;
use crate::backend::{FRBackend, IDSet, MatchConfig};
use crate::remote::{RegistrationPair, Remote, SearchResult};
use crate::repo::EnrollmentMetadataRecord;

use crate::EnrollData;
use crate::{
    DeleteFaceResult, EnrolledFaceInfo, FRIdentity, FRResult, Face, IDPair, SearchBy, Template,
};

use bytes::Bytes;
use libtpass::api::TPassClient;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub enum RemoteRuntime {
    TPass(Arc<TPassClient>),
    Local(Arc<TPassClient>), //local means we do it ourself but using tpass as a placeholder
}

impl RemoteRuntime {
    pub fn new(remote: &str, tpass_client: Arc<TPassClient>) -> Result<Self, String> {
        match remote {
            "tpass" => Ok(Self::TPass(tpass_client)),
            "local" => Ok(Self::Local(tpass_client)),
            _ => Err(format!("unsupported FR_REMOTE '{}'; supported values: tpass", remote)),
        }
    }
}

impl Remote for RemoteRuntime {
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.register_enrollment(reg_pair).await,
            Self::Local(client) => client.register_enrollment(reg_pair).await,
        }
    }

    async fn unregister_enrollment(&self) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.unregister_enrollment().await,
            Self::Local(client) => client.unregister_enrollment().await,
        }
    }

    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search(enroll_data).await,
            Self::Local(client) => client.search(enroll_data).await,
        }
    }

    async fn search_one(
        &self,
        search: SearchBy,
        include_image: bool,
    ) -> FRResult<Option<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_one(search, include_image).await,
            Self::Local(client) => client.search_one(search, include_image).await,
        }
    }

    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_by_ids(search, include_img).await,
            Self::Local(client) => client.search_by_ids(search, include_img).await,
        }
    }
}

#[derive(Clone)]
pub enum FREngine {
    Paravision(PVBackend),
}

impl FREngine {
    pub fn new(
        backend: &str,
        proc_url: String,
        ident_url: String,
        db: PgPool,
    ) -> Result<Self, String> {
        match backend {
            "paravision-grpc" | "paravision" | "pv" => {
                Ok(Self::Paravision(PVBackend::new(proc_url, ident_url, db)))
            }
            _ => Err(format!(
                "unsupported FR_BACKEND '{}'; supported values: paravision-grpc, paravision",
                backend
            )),
        }
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
        }
    }

    //TODO: indicate if we only want most prominent? or do after the fact?
    async fn generate_template(&self, image: Bytes) -> FRResult<Vec<Template>> {
        match self {
            Self::Paravision(backend) => backend.generate_template(image).await,
        }
    }

    async fn create_identity(&self, template: Template, ext_id: &str) -> FRResult<IDSet> {
        match self {
            Self::Paravision(backend) => backend.create_identity(template, ext_id).await,
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        match self {
            Self::Paravision(backend) => backend.get_enrollment_metadata().await,
        }
    }

    async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        match self {
            Self::Paravision(backend) => backend.detect_faces(image, liveness_check).await,
        }
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        match self {
            Self::Paravision(backend) => backend.recognize(image, config).await,
        }
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<EnrolledFaceInfo> {
        match self {
            Self::Paravision(backend) => backend.add_face(fr_id, image).await,
        }
    }

    async fn get_faces(&self, fr_id: &str) -> FRResult<Vec<EnrolledFaceInfo>> {
        match self {
            Self::Paravision(backend) => backend.get_faces(fr_id).await,
        }
    }

    async fn delete_faces(&self, fr_id: &str, face_ids: Vec<String>) -> FRResult<DeleteFaceResult> {
        match self {
            Self::Paravision(backend) => backend.delete_faces(fr_id, face_ids).await,
        }
    }
}
