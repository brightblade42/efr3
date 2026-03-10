use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use libfr::PossibleMatch;
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::Remote,
    repo::{EnrollmentMetadataRecord, ProfileRecord, SqlxFrRepository},
    DeleteFaceResult, EnrollData, EnrollDetails, EnrolledFaceInfo, EnrollmentDeleteResult,
    EnrollmentRosterItem, FRError, FRIdentity, FRResult, Face, IDPair, SearchBy,
};
use libtpass::types::TPassProfile;
use serde_json::{json, Value};
use tracing::{info, warn};

//use crate::recognition_handlers::RecognizeOpts;
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

    fn extract_and_validate_data(
        enroll_data: &EnrollData,
    ) -> FRResult<(&EnrollDetails, String, Bytes)> {
        //TODO: should we log any of these?
        let details = enroll_data.details.as_ref().ok_or_else(|| {
            FRError::with_code(
                1011,
                "enroll_details_error",
                "Enrollment details were not provided or could not be resolved",
            )
        })?;

        let ext_id = Self::extract_ext_id(&details).ok_or_else(|| {
            FRError::with_code(
                1050,
                "ext_id_missing_error",
                "An external id is required for create enrollment",
            )
        })?;

        //do we need this check?
        let image = enroll_data.image.clone().ok_or_else(|| {
            FRError::with_code(
                1011,
                "enroll_details_error",
                "Enrollment details were not provided or could not be resolved",
            )
        })?;

        Ok((details, ext_id, image))
    }

    //combines a duplicate check and a quality check.
    //return a face that can be used for enrollment
    async fn ensure_enrollable(&self, image: Bytes, config: MatchConfig) -> FRResult<Face> {
        //check threshold as well.
        self.duplicate_check(image.clone(), config).await?; //its OK to clone bytes::Bytes, cheap not deep

        //only ever enroll 1 face.
        let mut face = self.get_closest_face(image, false).await?;
        let quality = face.quality.unwrap_or(0.0);
        let acceptability = face.acceptability.unwrap_or(0.0);

        //we don't want these logged. waste
        //face.template = None;
        face.bbox = None;
        face.liveness = None;

        if quality <= config.min_quality || acceptability <= config.min_acceptability {
            let fr_err = FRError::with_details(
                1081,
                "low_quality_error",
                "min quality not met",
                json!(face),
            );

            self.log_enrollment_error("create_enrollment", &fr_err).await?;
            return Err(fr_err);
        }
        Ok(face)
    }

    //really just returns the most prominent face which is known to have quality attributes
    pub async fn get_closest_face(&self, image: Bytes, include_liveness: bool) -> FRResult<Face> {
        Ok(self
            .detect_faces(image, include_liveness)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| FRError::with_code(1081, "faces_not_found_error", "no faces found"))?)
    }

    pub async fn create_enrollment(
        &self,
        enroll_data: &EnrollData,
        config: MatchConfig,
    ) -> FRResult<IDPair> {
        let (details, ext_id, image) = Self::extract_and_validate_data(enroll_data)?;
        //quality and dupe check
        let face = self.ensure_enrollable(image, config).await?; //only care about early error return otherwise we know we're good to go
        let profile = Self::build_profile_record(&ext_id, None, &details);
        //the whole thing passess or fails.
        self.persist_profile(&profile).await?;

        //should be called create_identity?
        let response = match self.fr_engine.create_enrollment(&face, config, &ext_id).await {
            Ok(response) => response,
            Err(err) => {
                warn!(
                    "create enrollment \n{}",
                    serde_json::to_string_pretty(err.details.as_ref().unwrap_or_default())
                        .unwrap_or_default()
                );
                //TODO: validate proper logging
                self.log_enrollment_error("create_enrollment", &err).await?;
                return Err(err);
            }
        };

        Ok(response)
    }

    pub async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult> {
        //NOTE: delete enrollment is handled purely by deleting a profile.
        //we may want to rename it delete enrollment? maybe..
        match self.fr_repo.delete_profile_by_fr_id(fr_id).await {
            Ok(rows_affected) => {
                if rows_affected == 0 {
                    info!("delete_enrollment: {} not found", fr_id);
                    return Err(FRError::with_details(
                        1060,
                        "delete_enrollment_error",
                        "enrollment doesn't exist for fr_id",
                        json!({
                        "fr_id": fr_id
                        }),
                    ));
                }
            }
            Err(e) => {
                let details = json!({
                    "fr_id": fr_id,
                    "error": e.to_string(),
                });
                self.append_repo_log("delete_enrollment_error", details.clone()).await;
                return Err(FRError::with_details(
                    1060,
                    "delete_enrollment_error",
                    "Enrollment deleted in backend but eyefr profile cleanup failed",
                    details,
                ));
            }
        }

        //TODO: is this still a thing?
        self.remote.unregister_enrollment().await?;
        Ok(EnrollmentDeleteResult { fr_id: fr_id.to_string() })
    }

    pub async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        self.fr_repo.get_enrollment_metadata().await.map_err(|e| {
            FRError::with_details(
                1062,
                "load_enrollment_metadata_error",
                "Failed to load enrollment metadata",
                json!({ "error": e.to_string() }),
            )
        })
    }

    pub async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>> {
        let roster = self.fr_repo.get_enrollment_roster(1000).await.map_err(|e| {
            FRError::with_details(
                1064,
                "get_enrollment_roster_error",
                "Failed to load enrollment roster",
                json!({ "error": e.to_string() }),
            )
        })?;

        Ok(roster.into_iter().map(Self::profile_to_enrollment_item).collect())
    }

    pub async fn reset_enrollments(&self) -> FRResult<u64> {
        //let backend_result = self.fr_engine.reset_enrollments().await?;

        Ok(self.fr_repo.reset_enrollments().await.map_err(|e| {
            FRError::with_details(
                1065,
                "reset_enrollments_error",
                "failed to delete all enrollments",
                json!({ "error": e.to_string() }),
            )
        })?)
    }

    pub async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        self.fr_engine.detect_faces(image, liveness_check).await
    }

    pub async fn duplicate_check(&self, image: Bytes, config: MatchConfig) -> FRResult<()> {
        //no results means no duplicate, could be a clean db
        let fr_ident = self.fr_engine.recognize(image, config).await?;

        if fr_ident.is_empty() {
            return Ok(());
        }

        //not matter what, if there is a face, there is always a possible match. it's the closeness of that
        //match that matters
        let pm = fr_ident
            .into_iter()
            .next()
            .unwrap()
            .possible_matches
            .into_iter()
            .next()
            .ok_or_else(|| {
                FRError::with_code(
                    1081,
                    "duplicate_check_error",
                    "duplicate_check: image processing returned no possible matches",
                )
            })?;

        if pm.score >= config.min_dupe_match {
            //log dupe error
            let fr_err =
                FRError::with_details(1081, "duplicate_error", "duplicate_found", json!(pm));
            self.log_enrollment_error("create_enrollment", &fr_err).await?;
            return Err(fr_err);
        }

        Ok(())
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

        //TODO: skip if we want a local only call.

        if config.include_details {
            info!("include detail activated");
            let remote_matches =
                self.remote.search_by_ids(SearchBy::ExtIDS(ext_ids), false).await?;

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
        }
        Ok(fr_identities)
    }

    pub async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<EnrolledFaceInfo> {
        self.fr_engine.add_face(fr_id, image).await
    }

    pub async fn delete_faces(
        &self,
        fr_id: &str,
        face_ids: Vec<String>,
    ) -> FRResult<DeleteFaceResult> {
        self.fr_engine.delete_faces(fr_id, face_ids).await
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
                "search_profiles_error",
                "Failed to search enrollments by last name from eyefr repository",
                json!({ "term": term, "error": e.to_string() }),
            )
        })?;

        Ok(profiles.into_iter().map(Self::profile_to_enrollment_item).collect())
    }
    pub async fn log_cam_fr_match(
        &self,
        pm: &PossibleMatch,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        match self.fr_repo.log_cam_fr_match(pm, extra, location).await {
            Ok(_) => Ok(()),
            Err(e) => Err(FRError {
                code: 1000,
                name: "log_fr_cam_match_error".to_string(),
                message: e.to_string(),
                details: None,
            }),
        }
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
            //TODO: make sure this is the correct log method
            self.append_repo_log("profile_upsert_error", details.clone()).await;
            return Err(FRError::with_details(
                1053,
                "save_profile_error",
                "Failed to save profile snapshot",
                details,
            ));
        }

        Ok(())
    }

    // async fn persist_profile_snapshot(
    //     &self,
    //     ext_id: &str,
    //     details: &EnrollDetails,
    //     fr_id: Option<&str>,
    // ) -> FRResult<()> {
    //     let profile = Self::build_profile_record(ext_id, fr_id, details);

    //     if let Err(e) = self.fr_repo.upsert_profile(&profile).await {
    //         let details = json!({
    //             "fr_id": fr_id,
    //             "ext_id": ext_id,
    //             "error": e.to_string(),
    //         });
    //         self.append_repo_log("profile_upsert_error", details.clone()).await;
    //         return Err(FRError::with_details(
    //             1053,
    //             "Failed to persist enrollment profile snapshot",
    //             details,
    //         ));
    //     }

    //     Ok(())
    // }

    // async fn persist_image_snapshot(
    //     &self,
    //     ext_id: &str,
    //     image: &Bytes,
    //     details: &EnrollDetails,
    //     quality: f32,
    //     acceptability: f32,
    // ) -> FRResult<()> {
    //     let (_, _, _, img_url, raw_data) = Self::extract_profile_fields(Some(details));
    //     let image_record = ImageRecord {
    //         ext_id: ext_id.to_string(),
    //         data: image.to_vec(),
    //         size: Some(image.len() as f32),
    //         url: img_url,
    //         quality,
    //         acceptability,
    //         raw_data,
    //     };

    //     if let Err(e) = self.fr_repo.upsert_image(&image_record).await {
    //         let details = json!({
    //             "ext_id": ext_id,
    //             "error": e.to_string(),
    //         });
    //         self.append_repo_log("image_upsert_error", details.clone()).await;
    //         return Err(FRError::with_details(
    //             1054,
    //             "Failed to persist enrollment image snapshot",
    //             details,
    //         ));
    //     }

    //     Ok(())
    // }

    async fn log_enrollment_error(&self, code: &str, err: &FRError) -> FRResult<String> {
        warn!(code);
        warn!(err.message);
        let x = serde_json::to_string_pretty(&err.details).unwrap();
        warn!(x);
        Ok("logged enrollment error".to_string())
    }

    //TODO: replace this with log_enrollment_error
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
                img_url: None, //TODO: should this be filled in? file path or url?
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
