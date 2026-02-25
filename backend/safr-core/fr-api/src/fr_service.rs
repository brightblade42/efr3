use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::{RegistrationPair, Remote},
    repo::{
        EnrollmentMetadataRecord, ExternalId, ImageRecord, ProfileRecord, RegistrationErrorRecord,
        SqlxFrRepository,
    },
    AddFaceResult, DeleteFaceResult, EnrollData, EnrollDetails, EnrollmentCreateResult,
    EnrollmentDeleteResult, EnrollmentRosterItem, FRError, FRIdentity, FRResult, Face,
    GetFaceInfoResult, IDKind, Image, ResetEnrollmentsResult, SearchBy,
};
use serde_json::{json, Value};
use tracing::warn;

use crate::runtime::{FREngine, RemoteRuntime};

#[derive(Clone)]
pub struct FRService {
    fr_engine: Arc<FREngine>,
    remote: Arc<RemoteRuntime>,
    fr_repo: Arc<SqlxFrRepository>,
}

impl FRService {
    pub fn new(
        fr_engine: Arc<FREngine>,
        remote: Arc<RemoteRuntime>,
        fr_repo: Arc<SqlxFrRepository>,
    ) -> Self {
        Self { fr_engine, remote, fr_repo }
    }

    pub async fn create_enrollment(
        &self,
        mut enroll_data: EnrollData,
        config: MatchConfig,
    ) -> FRResult<EnrollmentCreateResult> {
        let remote_match = self.remote.search(&enroll_data).await?.into_iter().next();

        if enroll_data.image.is_none() {
            enroll_data.image = match remote_match.as_ref().and_then(|result| result.image.as_ref())
            {
                Some(Image::Binary(bin)) => Some(bin.clone()),
                None => None,
            };
        }

        let candidate_ext_id = remote_match
            .as_ref()
            .and_then(|result| result.id.clone())
            .or_else(|| enroll_data.details.as_ref().and_then(Self::extract_ext_id_from_details));

        let details_snapshot = enroll_data.details.clone();
        let image_snapshot = enroll_data.image.clone();

        let mut response = self
            .fr_engine
            .create_enrollment(enroll_data, config, candidate_ext_id.clone())
            .await?;

        let fr_id = response.fr_id.clone();

        let resolved_ext_id = if !response.ext_id_str.trim().is_empty() {
            response.ext_id_str.clone()
        } else if response.ext_id > 0 {
            response.ext_id.to_string()
        } else {
            candidate_ext_id.unwrap_or_default()
        };

        if resolved_ext_id.trim().is_empty() {
            return Err(FRError::with_code(
                1050,
                "External id was not found. Couldn't register with remote. Partial enrollment",
            ));
        }

        let external_id = ExternalId::new(resolved_ext_id.clone()).map_err(|e| {
            FRError::with_details(
                1052,
                "Enrollment succeeded but external id could not be normalized",
                json!({
                    "fr_id": fr_id,
                    "ext_id": resolved_ext_id,
                    "error": e.to_string(),
                }),
            )
        })?;

        response.ext_id_str = external_id.as_str().to_string();
        response.ext_id = external_id.as_str().parse::<u64>().unwrap_or(0);

        self.persist_profile_and_image(
            &external_id,
            &fr_id,
            details_snapshot.as_ref(),
            image_snapshot.as_ref(),
        )
        .await?;

        if external_id.as_str().parse::<u64>().is_ok() {
            let reg_pair = RegistrationPair::new(fr_id.clone(), external_id.as_str().to_string());
            if let Err(err) = self.remote.register_enrollment(&reg_pair).await {
                self.log_registration_failure(Some(&external_id), Some(&fr_id), &err).await;
                return Err(err);
            }
        } else {
            self.append_repo_log(
                "remote_registration_skipped",
                json!({
                    "fr_id": fr_id,
                    "ext_id": external_id.as_str(),
                }),
            )
            .await;
        }

        Ok(response)
    }

    pub async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult> {
        let response = self.fr_engine.delete_enrollment(fr_id).await?;

        match self.fr_repo.delete_profile_by_fr_id(fr_id).await {
            Ok(rows_affected) => {
                if rows_affected == 0 {
                    self.append_repo_log(
                        "profile_delete_miss",
                        json!({
                            "fr_id": fr_id,
                            "message": "No profile row matched fr_id during delete",
                        }),
                    )
                    .await;
                }
            }
            Err(e) => {
                let details = json!({
                    "fr_id": fr_id,
                    "error": e.to_string(),
                });
                self.append_repo_log("profile_delete_error", details.clone()).await;
                return Err(FRError::with_details(
                    1060,
                    "Enrollment deleted in backend but eyefr profile cleanup failed",
                    details,
                ));
            }
        }

        self.remote.unregister_enrollment().await?;
        Ok(response)
    }

    pub async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        self.fr_repo.get_enrollment_metadata().await.map_err(|e| {
            FRError::with_details(
                1062,
                "Failed to load enrollment metadata from eyefr repository",
                json!({ "error": e.to_string() }),
            )
        })
    }

    pub async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>> {
        let roster = self.fr_repo.get_enrollment_roster(1000).await.map_err(|e| {
            FRError::with_details(
                1064,
                "Failed to load enrollment roster from eyefr repository",
                json!({ "error": e.to_string() }),
            )
        })?;

        Ok(roster.into_iter().map(Self::profile_to_enrollment_item).collect())
    }

    pub async fn reset_enrollments(&self) -> FRResult<ResetEnrollmentsResult> {
        let backend_result = self.fr_engine.reset_enrollments().await?;

        let reset = self.fr_repo.reset_enrollment_state().await.map_err(|e| {
            FRError::with_details(
                1065,
                "PV reset succeeded but local eyefr reset failed",
                json!({ "error": e.to_string() }),
            )
        })?;

        Ok(ResetEnrollmentsResult { msg: backend_result.msg, local_reset: reset })
    }

    pub async fn detect_face(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        self.fr_engine.detect_face(image, liveness_check).await
    }

    pub async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let mut fr_identities = self.fr_engine.recognize(image, config).await?;

        let ccodes: Vec<u64> = fr_identities
            .iter()
            .flat_map(|fr| &fr.possible_matches)
            .filter_map(|pm| pm.ext_id.trim().parse::<u64>().ok())
            .filter(|ccode| *ccode != 0)
            .collect();

        if ccodes.is_empty() {
            return Ok(fr_identities);
        }

        let search_ids: Vec<IDKind> = ccodes
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .map(IDKind::Num)
            .collect();

        let remote_matches = self.remote.search_many(SearchBy::ExtIDS(search_ids), false).await?;

        let details_by_ccode: HashMap<u64, Value> = remote_matches
            .into_iter()
            .filter_map(|item| {
                let details = item.details?;
                let ccode = item
                    .id
                    .as_deref()
                    .and_then(|id| id.trim().parse::<u64>().ok())
                    .or_else(|| details.get("ccode").and_then(Value::as_u64))?;
                Some((ccode, details))
            })
            .collect();

        if details_by_ccode.is_empty() {
            return Ok(fr_identities);
        }

        for fr_ident in &mut fr_identities {
            for possible_match in &mut fr_ident.possible_matches {
                if let Some(ccode) =
                    possible_match.ext_id.trim().parse::<u64>().ok().filter(|ccode| *ccode != 0)
                {
                    if let Some(details) = details_by_ccode.get(&ccode) {
                        possible_match.details = Some(details.clone());
                    }
                }
            }
        }

        Ok(fr_identities)
    }

    pub async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<AddFaceResult> {
        self.fr_engine.add_face(fr_id, image).await
    }

    pub async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<DeleteFaceResult> {
        self.fr_engine.delete_face(fr_id, face_id).await
    }

    pub async fn get_face_info(&self, fr_id: &str) -> FRResult<GetFaceInfoResult> {
        self.fr_engine.get_face_info(fr_id).await
    }

    pub async fn get_enrollments_by_last_name(
        &self,
        name: &str,
    ) -> FRResult<Vec<EnrollmentRosterItem>> {
        let term = name.trim();
        if term.is_empty() {
            return Ok(vec![]);
        }

        let profiles = self.fr_repo.search_profiles_by_last_name(term, 100).await.map_err(|e| {
            FRError::with_details(
                1061,
                "Failed to search enrollments from eyefr repository",
                json!({ "term": term, "error": e.to_string() }),
            )
        })?;

        Ok(profiles.into_iter().map(Self::profile_to_enrollment_item).collect())
    }
    pub async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.fr_engine.log_identity(fr_identity, extra, location).await
    }

    async fn persist_profile_and_image(
        &self,
        external_id: &ExternalId,
        fr_id: &str,
        details: Option<&EnrollDetails>,
        image: Option<&Bytes>,
    ) -> FRResult<()> {
        let (first_name, last_name, middle_name, image_url, raw_data) =
            Self::extract_profile_fields(details);

        let profile = ProfileRecord {
            external_id: external_id.clone(),
            first_name,
            last_name,
            middle_name,
            image_url: image_url.clone(),
            raw_data: raw_data.clone(),
            fr_id: Some(fr_id.to_string()),
        };

        if let Err(e) = self.fr_repo.upsert_profile(&profile).await {
            let details = json!({
                "fr_id": fr_id,
                "ext_id": external_id.as_str(),
                "error": e.to_string(),
            });
            self.append_repo_log("profile_upsert_error", details.clone()).await;
            return Err(FRError::with_details(
                1053,
                "Enrollment succeeded but eyefr profile persistence failed",
                details,
            ));
        }

        if let Some(image_bytes) = image {
            let image_record = ImageRecord {
                external_id: external_id.clone(),
                data: image_bytes.to_vec(),
                size: Some(image_bytes.len() as f32),
                url: image_url,
                quality: 0.0,
                acceptability: 0.0,
                raw_data,
            };

            if let Err(e) = self.fr_repo.upsert_image(&image_record).await {
                let details = json!({
                    "fr_id": fr_id,
                    "ext_id": external_id.as_str(),
                    "error": e.to_string(),
                });
                self.append_repo_log("image_upsert_error", details.clone()).await;
                return Err(FRError::with_details(
                    1054,
                    "Enrollment succeeded but eyefr image persistence failed",
                    details,
                ));
            }
        }

        Ok(())
    }

    async fn log_registration_failure(
        &self,
        external_id: Option<&ExternalId>,
        fr_id: Option<&str>,
        err: &FRError,
    ) {
        let record = RegistrationErrorRecord {
            external_id: external_id.cloned(),
            fr_id: fr_id.map(str::to_string),
            message: Some(err.message.clone()),
        };

        if let Err(e) = self.fr_repo.insert_registration_error(&record).await {
            warn!("failed to record registration error in eyefr: {}", e);
        }

        let payload = json!({
            "ext_id": external_id.map(ExternalId::as_str),
            "fr_id": fr_id,
            "code": err.code,
            "message": err.message,
            "details": err.details.clone(),
        });
        self.append_repo_log("remote_registration_error", payload).await;
    }

    async fn append_repo_log(&self, code: &str, payload: Value) {
        if let Err(e) = self.fr_repo.append_enrollment_log(code, &payload).await {
            warn!("failed to append enrollment log '{}' to eyefr: {}", code, e);
        }
    }

    fn extract_profile_fields(
        details: Option<&EnrollDetails>,
    ) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<Value>) {
        match details {
            Some(EnrollDetails::Min { first_name, last_name }) => {
                (Some(first_name.clone()), Some(last_name.clone()), None, None, None)
            }
            Some(EnrollDetails::TPass(raw)) => (
                Self::pick_string(raw, &["first_name", "firstName", "firstname"]),
                Self::pick_string(raw, &["last_name", "lastName", "lastname"]),
                Self::pick_string(raw, &["middle_name", "middleName", "middlename"]),
                Self::pick_string(raw, &["imgUrl", "img_url", "imageUrl", "image_url"]),
                Some(raw.clone()),
            ),
            None => (None, None, None, None, None),
        }
    }

    fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|key| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
        })
    }

    fn profile_to_enrollment_item(profile: ProfileRecord) -> EnrollmentRosterItem {
        let details = profile.raw_data.clone().unwrap_or_else(|| {
            json!({
                "first_name": profile.first_name,
                "last_name": profile.last_name,
                "middle_name": profile.middle_name,
                "img_url": profile.image_url,
            })
        });

        EnrollmentRosterItem {
            fr_id: profile.fr_id,
            ext_id: profile.external_id.as_str().parse::<u64>().unwrap_or(0),
            ext_id_str: profile.external_id.as_str().to_string(),
            details,
        }
    }

    fn extract_ext_id_from_details(details: &EnrollDetails) -> Option<String> {
        match details {
            EnrollDetails::TPass(data) => {
                data.get("ccode").and_then(Value::as_u64).map(|num| num.to_string())
            }
            EnrollDetails::Min { .. } => None,
        }
    }
}
