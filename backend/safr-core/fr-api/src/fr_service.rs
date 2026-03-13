use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use libfr::remote::RegistrationPair;
use libfr::PossibleMatch;
use libfr::{
    backend::{FRBackend, MatchConfig},
    errors::FRError,
    remote::Remote,
    repo::{EnrollmentMetadataRecord, ProfileRecord, SqlxFrRepository},
    DeleteFaceResult, EnrollData, EnrollDetails, EnrolledFaceInfo, EnrollmentDeleteResult,
    FRIdentity, FRResult, Face, IDPair, SearchBy,
};
use libtpass::types::TPassProfile;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

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
        let details = enroll_data.details.as_ref().ok_or_else(|| FRError::Generic {
            code: "CREATE_ENROLLMENT_ERR".into(),
            message: "Enrollment details were not provided or could not be resolved".into(),
            details: None,
        })?;

        let ext_id = Self::extract_ext_id(&details).ok_or_else(|| FRError::Generic {
            code: "CREATE_ENROLLMENT_ERR".into(),
            message: "An external id is required to create an enrollment".into(),
            details: None,
        })?;

        //do we need this check?
        let image = enroll_data.image.clone().ok_or_else(|| FRError::Generic {
            code: "CREATE_ENROLLMENT_ERR".into(),
            message: "an image was not found in provided enrollment details".into(),
            details: None,
        })?;

        Ok((details, ext_id, image))
    }

    //combines a duplicate check and a quality check.
    //return a face that can be used for enrollment
    async fn ensure_enrollable(&self, image: Bytes, config: MatchConfig) -> FRResult<Face> {
        //check threshold as well.
        self.duplicate_check(image.clone(), config).await?;
        let mut face = self.get_closest_face(image, false).await?;
        let quality = face.quality.unwrap_or(0.0);
        let acceptability = face.acceptability.unwrap_or(0.0);

        //we don't want these logged. waste
        face.bbox = None;
        face.liveness = None;

        if quality <= config.min_quality || acceptability <= config.min_acceptability {
            return Err(FRError::PoorQuality { quality, min_quality: config.min_quality });
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
            .ok_or_else(|| FRError::FaceNotFound)?)
    }

    //makes a face recognizable by the system
    pub async fn create_enrollment(
        &self,
        enroll_data: &EnrollData,
        config: MatchConfig,
    ) -> FRResult<IDPair> {
        //info!(target: "create_enrollment", )
        let (details, ext_id, image) = Self::extract_and_validate_data(enroll_data)?;
        let face = self.ensure_enrollable(image, config).await; //only care about early error return otherwise we know we're good to go

        if let Err(ref e) = face {
            self.log_enrollment_error("create_enrollment", details, &e).await;
        }
        let face = face?;

        let profile = Self::build_profile_record(&ext_id, None, details);
        //the whole thing passess or fails. we save it before we have an identity then
        //it is updated with the new identity.
        self.persist_profile(&profile).await?;

        //NOTE: should be called create_identity? it's too late for that
        let id_pair = match self.fr_engine.create_enrollment(&face, config, &ext_id).await {
            Ok(ids) => ids,
            Err(err) => {
                self.log_enrollment_error("create_enrollment", details, &err).await; //.await?;
                return Err(err);
            }
        };

        //TODO: IDPair and RegistrationPair types are redundant, pick one
        let reg_pair = RegistrationPair::new(id_pair.fr_id.clone(), id_pair.ext_id.clone());
        if let Err(e) = self.remote.register_enrollment(&reg_pair).await {
            self.log_enrollment_error("create_enrollment", details, &e).await; //.await?;
            return Err(e);
        }

        info!(target: "create_enrollment", "Enrollment registered with Remote: ext_id: {} fr_id: {}", &reg_pair.ext_id, &reg_pair.fr_id);

        Ok(id_pair)
    }

    pub async fn delete_enrollment(&self, fr_id: &str) -> FRResult<EnrollmentDeleteResult> {
        //NOTE: delete enrollment is handled purely by deleting a profile.
        //we may want to rename it delete enrollment? maybe..
        match self.fr_repo.delete_profile_by_fr_id(fr_id).await {
            Ok(rows_affected) => {
                if rows_affected == 0 {
                    return Err(FRError::DeleteEnrollment {
                        fr_id: fr_id.into(),
                        message: "enrollment doesn't exist".into(),
                    });
                }
            }
            Err(e) => return Err(FRError::from(e)),
        }

        debug!("a debug message, bruh. clean me up");

        //TODO: is this still a thing?
        self.remote.unregister_enrollment().await?;
        Ok(EnrollmentDeleteResult { fr_id: fr_id.to_string() })
    }

    pub async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        self.fr_repo.get_enrollment_metadata().await.map_err(|e| FRError::from(e))
    }

    pub async fn get_roster(&self) -> FRResult<Vec<Value>> {
        let res = self.fr_repo.get_roster(1000).await.map_err(|e| FRError::from(e))?;

        Ok(Self::profiles_to_values(res))
    }

    pub fn profiles_to_values(profs: Vec<ProfileRecord>) -> Vec<Value> {
        profs
            .into_iter()
            .map(|p| {
                json!({

                    "fr_id": p.fr_id,
                    "ext_id": p.ext_id,
                    "first_name": p.first_name,
                    "last_name": p.last_name,
                    "middle_name": p.middle_name,
                    "img_url": p.img_url,

                })
            })
            .collect()
    }

    //NOTE: danger will robinson!
    pub async fn reset_enrollments(&self) -> FRResult<u64> {
        self.fr_repo.reset_enrollments().await.map_err(|e| FRError::from(e))
    }

    pub async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        self.fr_engine.detect_faces(image, liveness_check).await
    }

    pub async fn duplicate_check(&self, image: Bytes, config: MatchConfig) -> FRResult<()> {
        let fr_idents = self.fr_engine.recognize(image, config).await?;

        if fr_idents.is_empty() {
            return Ok(());
        }

        let pm = first_or_else!(
            fr_idents,
            possible_matches,
            FRError::Engine("dupe_check: no possible matches found.".into())
        );

        if pm.score >= config.min_dupe_match {
            return Err(FRError::Duplicate { ext_id: pm.ext_id, fr_id: pm.fr_id, score: pm.score });
        }

        Ok(())
    }

    pub async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let mut fr_identities = self.fr_engine.recognize(image, config).await?;

        //extract and dedupe External IDs. not really needed but eh.
        let ext_ids: std::collections::HashSet<String> = fr_identities
            .iter()
            .flat_map(|fr| &fr.possible_matches)
            .map(|pm| pm.ext_id.trim())
            .filter(|id| !id.is_empty())
            .map(|id| id.to_string())
            .collect();

        // we got nuttin? bail
        if ext_ids.is_empty() {
            return Ok(fr_identities);
        }

        // Add profile details if requested
        if config.include_details {
            let remote_matches = self
                .remote
                .search_by_ids(SearchBy::ExtIDS(ext_ids.into_iter().collect()), false)
                .await?;

            // Map remote profiles by their ID for lookup over loops
            let pmatch_profiles: HashMap<String, TPassProfile> =
                remote_matches.into_iter().filter_map(|sr| sr.id.zip(sr.details)).collect();

            // Patch the identities with the fresh remote data
            for possible_match in fr_identities.iter_mut().flat_map(|fi| &mut fi.possible_matches) {
                let key = possible_match.ext_id.trim();

                if let Some(profile) = pmatch_profiles.get(key) {
                    if let Ok(v) = serde_json::to_value(profile) {
                        possible_match.details = Some(v);
                    }
                }
            }
        }

        self.log_recognition(&fr_identities, config);
        Ok(fr_identities)
    }

    fn log_recognition(&self, idents: &[FRIdentity], config: MatchConfig) {
        for ident in idents {
            // first match or bust
            let Some(pm) = ident.possible_matches.first() else {
                continue;
            };

            // If we want details and they exist, log the full profile
            //NOTE: TPASS focuses, what will we do when we have other remotes, we may need to ditch the
            // generalized Value for profiles
            if config.include_details {
                if let Some(details) = pm.details.as_ref() {
                    // Use our new macro to grab strings safely
                    let f_name = json_str!(details, "fName");
                    let m_name = json_str!(details, "mName");
                    let l_name = json_str!(details, "lName");
                    let kind = json_str!(details, "type");
                    let status = json_str!(details, "status");

                    info!(target: "recognize",
                        "👤 score: {} | {} {} | {} {} {} | {} {}",
                        pm.score,pm.ext_id, pm.fr_id, f_name, m_name, l_name, kind, status,                     );
                }
            } else {
                info!(target: "recognize", "👤 score: {} | {} {} | [No Details] ", pm.score, pm.ext_id, pm.fr_id);
            }
        }
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

    pub async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
        let term = name.trim();
        if term.is_empty() {
            return Ok(vec![]);
        }

        let profiles = self
            .fr_repo
            .search_profiles_by_last_name(term, 100)
            .await
            .map_err(|e| FRError::from(e))?;

        Ok(Self::profiles_to_values(profiles))
    }
    pub async fn log_cam_fr_match(
        &self,
        pm: &PossibleMatch,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.fr_repo
            .log_cam_fr_match(pm, extra, location)
            .await
            .map_err(|e| FRError::from(e))
    }

    async fn persist_profile(&self, profile: &ProfileRecord) -> FRResult<()> {
        let ext_id = profile.ext_id.clone();

        if let Err(e) = self.fr_repo.upsert_profile(&profile).await {
            return Err(FRError::SaveProfile { ext_id: ext_id, message: e.to_string() });
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

    async fn log_enrollment_error(
        &self,
        code: &str,
        enroll_details: &EnrollDetails,
        err: &FRError,
    ) {
        warn!(target: "enrollment", "{} {} ", err, Self::format_details_for_log(enroll_details));
        let json_err = serde_json::to_value(err).unwrap_or(json!({}));
        let json_input = serde_json::to_value(enroll_details).unwrap_or(json!({}));

        //Database Write (Don't let it kill the app if it fails)
        let db_res = self.fr_repo.log_enrollment_errors(&[code], &[json_err], &[json_input]).await;

        if let Err(db_err) = db_res {
            // If the DB log fails, we log THAT to stdout too but we don't stop the application.
            error!(target: "enrollment", "‼️ CRITICAL: Could not write error log to Postgres: {}", db_err);
        }
    }

    fn format_details_for_log(details: &EnrollDetails) -> String {
        match details {
            EnrollDetails::Min { ext_id, last_name, first_name } => {
                // as_deref() turns Option<String> into Option<&str>
                let id = ext_id.as_deref().unwrap_or("unknown");
                format!("| ext_id: {} | {} {}", id, first_name, last_name)
            }
            EnrollDetails::TPass(prof) => {
                let id = prof.ccode.unwrap_or(0);
                let fname = prof.f_name.as_deref().unwrap_or("none");
                let lname = prof.l_name.as_deref().unwrap_or("none");

                format!("| ext_id: {} | {} {}", id, fname, lname)
            }
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
