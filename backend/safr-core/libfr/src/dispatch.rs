//! Runtime dispatch traits and implementation selectors.
//!
//! This module defines the stable seams between the orchestration layer and the concrete
//! integrations behind it:
//! - [`FRBackend`] for face-engine operations such as recognition and enrollment
//! - [`AssetStore`] for remote-profile and attendance-related operations
//!
//! `FRService` depends on these traits so the higher-level workflow code stays mostly agnostic to
//! the selected backend. In this repository the active implementations are Paravision for the FR
//! engine and TPass for the remote asset store.
use crate::errors::FRError;
use crate::pv::PVBackend;
use crate::repo::EnrollmentMetadataRecord;
use crate::types::{
    DeleteFaceResult, EnrollData, EnrolledFaceInfo, FRIdentity, FRResult, Face, IDPair,
    MatchConfig, SearchBy, SearchResult, Template,
};
use serde_json::{Value, json};
use tracing::info;

use crate::tpass::api::TPassClient;
use crate::tpass::config::TPassConf;
use crate::tpass::types::{AttendanceKind, AttendanceStatus, FRAlert};
use bytes::Bytes;
use sqlx::PgPool;
use std::sync::Arc;

/// Remote system of record for person/profile data associated with FR identities.
#[allow(async_fn_in_trait)]
pub trait AssetStore: Send + Sync {
    /// Register a newly created FR enrollment with the remote system.
    async fn register_enrollment(&self, id_pair: &IDPair) -> FRResult<()>;
    /// Undo remote registration when an enrollment is deleted.
    async fn unregister_enrollment(&self) -> FRResult<()>;
    /// Search the remote system using enrollment input such as image or profile details.
    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>>;
    /// Resolve at most one remote record for a given search key.
    async fn search_one(
        &self,
        search: SearchBy,
        include_image: bool,
    ) -> FRResult<Option<SearchResult>>;
    /// Resolve many remote records by one or more identifiers.
    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>>;
    //async fn create_profile(&self, some_profile_info) -> FRResult;
    //need mark_attendance?

    async fn mark_attendance(
        &self,
        idpair: (String, u64),
        att_kind: AttendanceKind,
    ) -> FRResult<Option<AttendanceStatus>>;

    async fn send_fr_alert(&self, alert: FRAlert) -> FRResult<Value>;
}

/// Runtime selector for the active remote implementation.
#[derive(Clone)]
pub enum AssetDispatcher {
    TPass(Arc<TPassClient>),
    Local(Option<String>), //local means we do it ourself but using tpass as a placeholder
}

impl AssetDispatcher {
    /// Build the configured remote dispatcher from an env-derived remote name.
    pub fn new(remote: &str, tpass_client: Arc<TPassClient>) -> Result<Self, String> {
        match remote {
            "tpass" => Ok(Self::TPass(tpass_client)),
            "local" => Ok(Self::Local(None)),
            _ => Err(format!("unsupported FR_REMOTE '{}'; supported values: tpass", remote)),
        }
    }

    pub fn from_env(asset_name: &str) -> Result<Self, String> {
        match asset_name {
            "tpass" => {
                //load TPASS from environment vars.
                Ok(Self::TPass(Arc::new(TPassClient::new(TPassConf::from_env()))))
            }

            "local" => Ok(Self::Local(None)),
            _ => Err(format!(
                "unsupported FR_REMOTE '{}'; supported values: tpass, local",
                asset_name
            )),
        }
    }
}

impl AssetStore for AssetDispatcher {
    async fn register_enrollment(&self, id_pair: &IDPair) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.register_enrollment(id_pair).await,
            Self::Local(_) => Ok(()),
        }
    }

    async fn unregister_enrollment(&self) -> FRResult<()> {
        match self {
            Self::TPass(client) => client.unregister_enrollment().await,
            Self::Local(_) => Ok(()),
        }
    }

    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search(enroll_data).await,
            Self::Local(_) => Ok(vec![]),
        }
    }

    async fn search_one(
        &self,
        search: SearchBy,
        include_image: bool,
    ) -> FRResult<Option<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_one(search, include_image).await,
            Self::Local(_) => Ok(None),
        }
    }

    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>> {
        match self {
            Self::TPass(client) => client.search_by_ids(search, include_img).await,
            Self::Local(_) => Ok(vec![]),
        }
    }

    async fn mark_attendance(
        &self,
        idpair: (String, u64),
        att_kind: AttendanceKind,
    ) -> FRResult<Option<AttendanceStatus>> {
        match self {
            Self::TPass(client) => {
                client.mark_attendance(idpair, att_kind).await.map_err(FRError::from)
            }
            Self::Local(_) => Ok(None),
        }
    }

    async fn send_fr_alert(&self, alert: FRAlert) -> FRResult<Value> {
        match self {
            Self::TPass(client) => client.send_fr_alert(alert).await.map_err(FRError::from),
            Self::Local(_) => Ok(json!({})),
        }
    }
}

/// Abstraction over FR engine capabilities used by the service layer.
#[allow(async_fn_in_trait)]
pub trait FRBackend: Send + Sync {
    /// Create a primary enrollment from the best validated face in an input image.
    async fn create_enrollment(
        &self,
        face: &Face,
        config: MatchConfig,
        ext_id: &str,
    ) -> FRResult<IDPair>; //create an enrollment for a single face
    //async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult>; //delete an enrollment for a singel face
    /// Return aggregate counts used by admin and operator views.
    async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord>;
    //async fn reset_enrollments(&self) -> FRResult<ResetEnrollmentsBackendResult>; //delete the whole damn thing. away with you.
    /// Detect faces in an image, optionally requesting liveness data.
    async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>>;
    /// Recognize one or more faces in an image and return ranked candidate matches.
    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>>;

    /// Generate templates from raw image data when the backend supports it.
    async fn generate_template(&self, image: Bytes) -> FRResult<Vec<Template>>;
    /// Create an identity directly from a template.
    async fn create_identity(&self, template: Template, ext_id: &str) -> FRResult<IDPair>;

    /// Add a secondary face to an existing identity.
    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<EnrolledFaceInfo>;
    /// Delete one or more stored face records from an identity.
    async fn delete_faces(&self, fr_id: &str, face_ids: Vec<String>) -> FRResult<DeleteFaceResult>;
    /// List the stored faces for an identity.
    async fn get_faces(&self, fr_id: &str) -> FRResult<Vec<EnrolledFaceInfo>>;
    //async fn get_face_info(&self, fr_id: &str) -> FRResult<GetFaceInfoResult>;
}

/// Runtime selector for the active FR backend implementation.
#[derive(Clone)]
pub enum FRDispatcher {
    Paravision(PVBackend),
}

impl FRDispatcher {
    /// Build the configured FR backend from env-derived backend identifiers and endpoints.
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

impl FRBackend for FRDispatcher {
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

    async fn create_identity(&self, template: Template, ext_id: &str) -> FRResult<IDPair> {
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
