use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::Remote,
    repo::{
        EnrollmentMetadataRecord, ImageRecord, ProfileRecord, RegistrationErrorRecord,
        SqlxFrRepository,
    },
    AddFaceResult, DeleteFaceResult, EnrollData, EnrollDetails, EnrollmentDeleteResult,
    EnrollmentRosterItem, FRError, FRIdentity, FRResult, Face, GetFaceInfoResult, IDPair,
    ResetEnrollmentsResult, SearchBy,
};
use libtpass::api::TPassClient;
use libtpass::types::TPassProfile;
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
        enroll_data: &EnrollData,
        config: MatchConfig,
    ) -> FRResult<IDPair> {
        let details = enroll_data.details.as_ref().ok_or_else(|| {
            FRError::with_code(
                1011,
                "Enrollment details were not provided or could not be resolved",
            )
        })?;

        let ext_id = Self::extract_ext_id(&details).ok_or_else(|| {
            FRError::with_code(1050, "An external id is required for create enrollment")
        })?;

        //TODO: check for duplicate

        //TODO: do quality check

        let profile = Self::build_profile_record(&ext_id, None, &details);
        //the whole thing passess or fails.
        self.persist_profile(&profile).await?;

        //should be called create_identity?
        let response = match self.fr_engine.create_enrollment(enroll_data, config, &ext_id).await {
            Ok(response) => response,
            Err(err) => {
                //TODO: validate proper logging
                self.log_create_enrollment_failure(&ext_id, &err).await;
                return Err(err);
            }
        };

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

        //TODO: we may not always want to contact the remote for profile.
        // many times we'll just want the id pairs and this will waste time and resources
        // when it's not needed

        //we extract the ext ids into a flat list from all possible matches for each identity so that we can
        //get updated profile information for each possible match from the remote.
        //what we store locally isn't guaranteed to be up to date
        //or even exist depending on the privacy concerns.

        let ext_ids: Vec<String> = fr_identities
            .iter()
            .flat_map(|fr| &fr.possible_matches)
            .filter_map(|pm| {
                let ext_id = pm.ext_id.trim();
                (!ext_id.is_empty()).then(|| ext_id.to_string())
            })
            .collect();

        //we got no ids so just return what is very likely an empty vector, ie nothing was recognized.
        if ext_ids.is_empty() {
            return Ok(fr_identities);
        }

        let remote_matches = self.remote.search_by_ids(SearchBy::ExtIDS(ext_ids), false).await?;

        //now we need to reinsert the updated profile info into the details of each
        //possible match

        let pmatch_profiles: HashMap<String, TPassProfile> = remote_matches
            .into_iter()
            .filter_map(|sr| match (sr.id, sr.details) {
                (Some(id), Some(details)) => Some((id, details)),
                _ => None,
            })
            .collect();

        for fr_ident in &mut fr_identities {
            for possible_match in &mut fr_ident.possible_matches {
                //we wouldn't get there is ext was empty
                let key = possible_match.ext_id.trim();
                if key.is_empty() {
                    continue;
                }

                if let Some(details) = pmatch_profiles.get(key) {
                    let v = serde_json::to_value(details).unwrap();
                    possible_match.details = Some(v);
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

    async fn persist_profile(&self, profile: &ProfileRecord) -> FRResult<()> {
        let ext_id = profile.ext_id.clone();
        let fr_id = profile.fr_id.clone();

        if let Err(e) = self.fr_repo.upsert_profile(&profile).await {
            let details = json!({
                "fr_id": fr_id,
                "ext_id": ext_id,
                "error": e.to_string(),
            });
            self.append_repo_log("profile_upsert_error", details.clone()).await;
            return Err(FRError::with_details(
                1053,
                "Failed to persist enrollment profile snapshot",
                details,
            ));
        }

        Ok(())
    }

    async fn persist_profile_snapshot(
        &self,
        ext_id: &str,
        details: &EnrollDetails,
        fr_id: Option<&str>,
    ) -> FRResult<()> {
        let profile = Self::build_profile_record(ext_id, fr_id, details);

        if let Err(e) = self.fr_repo.upsert_profile(&profile).await {
            let details = json!({
                "fr_id": fr_id,
                "ext_id": ext_id,
                "error": e.to_string(),
            });
            self.append_repo_log("profile_upsert_error", details.clone()).await;
            return Err(FRError::with_details(
                1053,
                "Failed to persist enrollment profile snapshot",
                details,
            ));
        }

        Ok(())
    }

    async fn persist_image_snapshot(
        &self,
        ext_id: &str,
        image: &Bytes,
        details: &EnrollDetails,
        quality: f32,
        acceptability: f32,
    ) -> FRResult<()> {
        let (_, _, _, img_url, raw_data) = Self::extract_profile_fields(Some(details));
        let image_record = ImageRecord {
            ext_id: ext_id.to_string(),
            data: image.to_vec(),
            size: Some(image.len() as f32),
            url: img_url,
            quality,
            acceptability,
            raw_data,
        };

        if let Err(e) = self.fr_repo.upsert_image(&image_record).await {
            let details = json!({
                "ext_id": ext_id,
                "error": e.to_string(),
            });
            self.append_repo_log("image_upsert_error", details.clone()).await;
            return Err(FRError::with_details(
                1054,
                "Failed to persist enrollment image snapshot",
                details,
            ));
        }

        Ok(())
    }

    //Codex went wild here
    async fn log_create_enrollment_failure(&self, ext_id: &str, err: &FRError) {
        //This is wrong AF
        let record = RegistrationErrorRecord {
            ext_id: Some(ext_id.to_string()),
            fr_id: None,
            message: Some(err.message.clone()),
        };

        //TODO: this doesn't belong here.
        if let Err(e) = self.fr_repo.insert_registration_error(&record).await {
            warn!("failed to record registration error in eyefr: {}", e);
        }

        let code = match err.code {
            1012 => "enrollment_quality_rejected",
            1020 => "enrollment_duplicate_rejected",
            _ => "enrollment_create_failed",
        };

        let payload = json!({
            "ext_id": ext_id,
            "code": err.code,
            "message": err.message,
            "details": err.details.clone(),
        });
        self.append_repo_log(code, payload).await;
    }

    async fn append_repo_log(&self, code: &str, payload: Value) {
        if let Err(e) = self.fr_repo.append_enrollment_log(code, &payload).await {
            warn!("failed to append enrollment log '{}' to eyefr: {}", code, e);
        }
    }

    fn build_profile_record(
        ext_id: &str,
        fr_id: Option<&str>,
        details: &EnrollDetails,
    ) -> ProfileRecord {
        match details.clone() {
            EnrollDetails::Min { first_name, last_name, .. } => ProfileRecord {
                ext_id: ext_id.to_string(),
                first_name: Some(first_name),
                last_name: Some(last_name),
                middle_name: None,
                img_url: None,
                raw_data: None,
                fr_id: fr_id.map(str::to_string),
            },

            EnrollDetails::TPass(prof) => ProfileRecord {
                ext_id: ext_id.to_string(),
                first_name: prof.f_name,
                last_name: prof.l_name,
                middle_name: prof.m_name,
                img_url: prof.img_url,
                raw_data: Some(json!(prof.raw_details)),
                fr_id: fr_id.map(str::to_string),
            },
        }
    }
    fn extract_profile_fields(
        details: Option<&EnrollDetails>,
    ) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<Value>) {
        match details {
            Some(EnrollDetails::Min { first_name, last_name, .. }) => {
                (Some(first_name.clone()), Some(last_name.clone()), None, None, None)
            }
            //Some(EnrollDetails::TPass(prof)) =>
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
                "img_url": profile.img_url,
            })
        });

        EnrollmentRosterItem { fr_id: profile.fr_id, ext_id: profile.ext_id, details }
    }

    fn extract_ext_id(details: &EnrollDetails) -> Option<String> {
        match details {
            EnrollDetails::TPass(prof) => prof.ccode.map(|c| c.to_string()),
            EnrollDetails::Min { ext_id, .. } => ext_id.as_deref().and_then(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
        }
    }
}
