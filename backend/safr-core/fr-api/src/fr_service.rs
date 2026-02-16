use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::{RegistrationPair, Remote},
    EnrollData, EnrollDetails, FRError, FRIdentity, FRResult, IDKind, Image, SearchBy,
};
use serde_json::{json, Value};

use crate::runtime::{FREngine, RemoteRuntime};

#[derive(Clone)]
pub struct FRService {
    fr_engine: Arc<FREngine>,
    remote: Arc<RemoteRuntime>,
}

impl FRService {
    pub fn new(fr_engine: Arc<FREngine>, remote: Arc<RemoteRuntime>) -> Self {
        Self { fr_engine, remote }
    }

    pub async fn create_enrollment(
        &self,
        mut enroll_data: EnrollData,
        config: MatchConfig,
    ) -> FRResult<Value> {
        let remote_match = self.remote.search(&enroll_data).await?.into_iter().next();

        if enroll_data.image.is_none() {
            enroll_data.image = match remote_match
                .as_ref()
                .and_then(|result| result.image.as_ref())
            {
                Some(Image::Binary(bin)) => Some(general_purpose::STANDARD.encode(bin)),
                Some(Image::Base64(b64)) => Some(b64.clone()),
                None => None,
            };
        }

        let ext_id = remote_match
            .as_ref()
            .and_then(|result| result.id)
            .or_else(|| {
                enroll_data
                    .details
                    .as_ref()
                    .and_then(Self::extract_ext_id_from_details)
            });

        let mut response = self
            .fr_engine
            .create_enrollment(enroll_data, config, ext_id)
            .await?;

        let fr_id = response
            .get("fr_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                FRError::with_code(
                    1051,
                    "Enrollment succeeded but response did not include fr_id",
                )
            })?
            .to_string();

        let ext_id = response
            .get("ext_id")
            .and_then(Value::as_u64)
            .filter(|id| *id != 0)
            .or(ext_id)
            .ok_or_else(|| {
                FRError::with_code(
                    1050,
                    "External id was not found. Couldn't register with remote. Partial enrollment",
                )
            })?;

        if let Some(resp_obj) = response.as_object_mut() {
            resp_obj.insert("ext_id".to_string(), json!(ext_id));
        }

        let reg_pair = RegistrationPair::new(fr_id, ext_id);
        self.remote.register_enrollment(&reg_pair).await?;

        Ok(response)
    }

    pub async fn delete_enrollment(&self, fr_id: &str) -> FRResult<Value> {
        let response = self.fr_engine.delete_enrollment(fr_id).await?;
        self.remote.unregister_enrollment().await?;
        Ok(response)
    }

    pub async fn get_enrollment_metadata(&self) -> FRResult<Value> {
        self.fr_engine.get_enrollment_metadata().await
    }

    pub async fn get_enrollment_roster(&self) -> FRResult<Value> {
        self.fr_engine.get_enrollment_roster().await
    }

    pub async fn reset_enrollments(&self) -> FRResult<Value> {
        self.fr_engine.reset_enrollments().await
    }

    pub async fn detect_face(&self, b64: String, spoof_check: bool) -> FRResult<Value> {
        self.fr_engine.detect_face(b64, spoof_check).await
    }

    pub async fn recognize(&self, b64: String, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let mut fr_identities = self.fr_engine.recognize(b64, config).await?;

        let ext_ids: Vec<IDKind> = fr_identities
            .iter()
            .flat_map(|fr| &fr.possible_matches)
            .filter(|pm| pm.ext_id != 0)
            .map(|pm| IDKind::Num(pm.ext_id))
            .collect();

        if ext_ids.is_empty() {
            return Ok(fr_identities);
        }

        let remote_matches = self
            .remote
            .search_many(SearchBy::ExtIDS(ext_ids), false)
            .await?;

        for fr_ident in &mut fr_identities {
            for possible_match in &mut fr_ident.possible_matches {
                for remote_match in &remote_matches {
                    if remote_match["ccode"]
                        .as_u64()
                        .is_some_and(|ccode| ccode == possible_match.ext_id)
                    {
                        possible_match.details = Some(remote_match.clone());
                    }
                }
            }
        }

        Ok(fr_identities)
    }

    pub async fn add_face(&self, fr_id: &str, b64: String) -> FRResult<Value> {
        self.fr_engine.add_face(fr_id, b64).await
    }

    pub async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value> {
        self.fr_engine.delete_face(fr_id, face_id).await
    }

    pub async fn get_face_info(&self, fr_id: &str) -> FRResult<Value> {
        self.fr_engine.get_face_info(fr_id).await
    }

    pub async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
        self.fr_engine.get_enrollments_by_last_name(name).await
    }

    pub async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.fr_engine
            .log_identity(fr_identity, extra, location)
            .await
    }

    fn extract_ext_id_from_details(details: &EnrollDetails) -> Option<u64> {
        match details {
            EnrollDetails::TPass(data) => data
                .get("ccode")
                .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok())),
            EnrollDetails::Min { .. } => None,
        }
    }
}
