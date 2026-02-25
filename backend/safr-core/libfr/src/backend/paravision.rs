use super::{
    pvtypes::{
        add_faces_request_from_processed, create_identities_request_from_processed,
        default_process_full_image_request, delete_faces_request, delete_identity_request,
        get_faces_request, identity_created_at, list_identities_request,
        liveness_process_full_image_request, lookup_candidates_from_processed,
        lookup_request_for_embedding, possible_matches_from_lookup, to_add_face_result,
        to_get_face_info_result,
    },
    FRBackend, FRResult, MatchConfig,
};
use crate::repo::EnrollmentMetadataRecord;
use crate::{
    utils, AddFaceResult, DeleteFaceResult, EnrollData, EnrollDetails, EnrollmentCreateResult,
    EnrollmentDeleteResult, EnrollmentRosterItem, FRError, FRIdentity, Face, GetFaceInfoResult,
    ResetEnrollmentsBackendResult,
};
use bytes::Bytes;
use libpv::identity_grpc::{identity, PVIdentityGrpcApi};
use libpv::proc_grpc::{processor, PVProcGrpcApi};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct PVBackend {
    proc_api: PVProcGrpcApi,
    ident_api: PVIdentityGrpcApi,
    db: PgPool,
}

#[derive(sqlx::FromRow)]
struct EnrollmentRosterRow {
    fr_id: Option<String>,
    ext_id: String,
    first_name: Option<String>,
    last_name: Option<String>,
    middle_name: Option<String>,
    img_url: Option<String>,
    raw_data: Option<Value>,
}

impl PVBackend {
    pub fn new(proc_url: String, ident_url: String, db: PgPool) -> Self {
        Self {
            proc_api: PVProcGrpcApi::new(proc_url),
            ident_api: PVIdentityGrpcApi::new(ident_url),
            db,
        }
    }

    async fn get_enrollable_face(
        &self,
        enroll_data: &EnrollData,
        config: MatchConfig,
    ) -> FRResult<identity::CreateIdentitiesRequest> {
        let image = enroll_data.image.clone().ok_or_else(|| {
            FRError::with_code(1010, "An image is required for enrollment but was not found")
        })?;

        let process_req = default_process_full_image_request(image, true);
        let img_resp = self.proc_api.process_full_image(process_req).await?;

        if img_resp.most_prominent_face_idx < 0 {
            return Err(FRError::with_code(
                1080,
                "enrollment image must be set to use most prominent face with was not set. ",
            ));
        }

        let p_idx = img_resp.most_prominent_face_idx as usize;
        let faces = if img_resp.faces.is_empty() { None } else { Some(&img_resp.faces) }
            .ok_or_else(|| {
                FRError::with_code(1081, "enrollment image processing returned no faces")
            })?;

        let prom_face = faces.get(p_idx).ok_or_else(|| {
            FRError::with_code(1082, "most prominent face index is out of bounds")
        })?;

        let quality = prom_face.quality;
        let acceptability = prom_face.acceptability;
        if quality < config.min_quality || acceptability < config.min_acceptability {
            let details = json!({
                "quality": quality,
                "acceptability": acceptability,
                "min_quality": config.min_quality,
                "min_acceptability": config.min_acceptability,
                "quality_pct": utils::score_to_percentage(quality),
                "acceptability_pct": utils::score_to_percentage(acceptability),
                "min_quality_pct": utils::score_to_percentage(config.min_quality),
                "min_acceptability_pct": utils::score_to_percentage(config.min_acceptability),
            });

            return Err(FRError::with_details(
                1012,
                "Image quality did not meet standards",
                details,
            ));
        }

        let emb = (!prom_face.embedding.is_empty())
            .then_some(prom_face.embedding.clone())
            .ok_or_else(|| {
                FRError::with_code(1083, "most prominent face did not include an embedding")
            })?;

        let lookup_req = lookup_request_for_embedding(emb, 1);
        let resp = self.ident_api.lookup(lookup_req).await?;
        let ident = resp.lookup_identities.first();

        match ident {
            Some(id) => {
                let duplicate = id.matches.iter().find(|ic| ic.score >= config.min_dupe_match);

                if let Some(duplicate_match) = duplicate {
                    let score = duplicate_match.score;
                    let score_pct = utils::score_to_percentage(score);
                    let threshold = config.min_dupe_match;
                    let threshold_pct = utils::score_to_percentage(threshold);

                    let (fr_id, created_at) = duplicate_match
                        .identity
                        .as_ref()
                        .map(|identity| (identity.id.clone(), identity_created_at(identity)))
                        .unwrap_or_else(|| (String::new(), String::new()));

                    let details = json!({
                        "fr_id": fr_id,
                        "created_at": created_at,
                        "score": score,
                        "score_pct": score_pct,
                        "threshold": threshold,
                        "threshold_pct": threshold_pct,
                    });

                    Err(FRError::with_details(
                        1020,
                        &format!("Duplicate: {:.2}% match", score_pct),
                        details,
                    ))
                } else {
                    Ok(create_identities_request_from_processed(
                        &img_resp,
                        config.min_dupe_match,
                        None,
                    ))
                }
            }
            None => {
                Ok(create_identities_request_from_processed(&img_resp, config.min_dupe_match, None))
            }
        }
    }

    async fn log_enroll_err(&self, action: &str, e: &FRError, details: &EnrollDetails) {
        error!("Enrollment error: {}", e.message);
        let e_val = match serde_json::to_value(e) {
            Ok(v) => v,
            Err(err) => {
                error!("failed to serialize enrollment error: {}", err);
                return;
            }
        };

        let details_val = match serde_json::to_value(details) {
            Ok(v) => v,
            Err(err) => {
                error!("failed to serialize enrollment details: {}", err);
                return;
            }
        };

        let res = sqlx::query(
            r" INSERT into paravision.enrollment_log (action,error,message,details) VALUES ($1,$2,$3,$4) ",
        )
        .bind(action)
        .bind(e_val)
        .bind(&e.message)
        .bind(details_val)
        .execute(&self.db)
        .await;

        if res.is_err() {
            error!("#GREAT SCOTT! The database write failed! Abandon all hope. we should panic.");
        }
    }

    async fn log_delete_err(&self, action: &str, e: &FRError, details: Option<Value>) {
        error!("Delete Enrollment error: {}", e.message);
        let e_val = match serde_json::to_value(e) {
            Ok(v) => v,
            Err(err) => {
                error!("failed to serialize delete error: {}", err);
                return;
            }
        };

        let details_val = match serde_json::to_value(details) {
            Ok(v) => v,
            Err(err) => {
                error!("failed to serialize delete details: {}", err);
                return;
            }
        };

        let res = sqlx::query(
            r" INSERT into paravision.enrollment_log (action,error,message,details) VALUES ($1,$2,$3,$4) ",
        )
        .bind(action)
        .bind(e_val)
        .bind(&e.message)
        .bind(details_val)
        .execute(&self.db)
        .await;

        if let Err(_lg_err) = res {
            error!("#GREAT SCOTT! The database write failed! Abandon all hope. we should panic.");
        }
    }

    async fn log_identification_db(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        let pm = fr_identity.possible_matches.first().ok_or_else(|| {
            FRError::with_code(
                2020,
                "could not log positive identification. identity has no possible matches",
            )
        })?;

        let confidence = pm.score;
        let pm_val = serde_json::to_value(pm)?;

        let res = sqlx::query(
            r"Insert into public.fr_log (pmatch, extra, location, confidence) VALUES ($1, $2, $3, $4)",
        )
        .bind(pm_val)
        .bind(extra)
        .bind(location)
        .bind(confidence)
        .execute(&self.db)
        .await;

        if let Err(err) = res {
            error!("#GREAT SCOTT! The database write failed for log_identification! {:?}", err);
            return Err(FRError::with_code(
                2021,
                "#GREAT SCOTT! The database write failed for log_identification!",
            ));
        }

        Ok(())
    }

    fn to_fr_identities(
        &self,
        faces: &[processor::Face],
        lookups: &[identity::LookupIdentity],
        min_match: f32,
    ) -> Vec<FRIdentity> {
        faces
            .iter()
            .enumerate()
            .map(|(idx, face)| FRIdentity {
                face: Face::from(face),
                possible_matches: lookups
                    .get(idx)
                    .map(possible_matches_from_lookup)
                    .unwrap_or_default(),
            })
            .filter(|fr_identity| {
                fr_identity
                    .possible_matches
                    .first()
                    .is_some_and(|possible_match| possible_match.score >= min_match)
            })
            .collect()
    }

    fn profile_to_enrollment_item(row: EnrollmentRosterRow) -> EnrollmentRosterItem {
        let details = row.raw_data.unwrap_or_else(|| {
            json!({
                "first_name": row.first_name,
                "last_name": row.last_name,
                "middle_name": row.middle_name,
                "img_url": row.img_url,
            })
        });

        EnrollmentRosterItem {
            fr_id: row.fr_id,
            ext_id: row.ext_id.parse::<u64>().unwrap_or(0),
            ext_id_str: row.ext_id,
            details,
        }
    }
}

impl FRBackend for PVBackend {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<String>,
    ) -> FRResult<EnrollmentCreateResult> {
        if enroll_data.image.is_none() {
            return Err(FRError::with_code(
                1010,
                "An image is required for enrollment but was not found",
            ));
        }

        let ident_req = self.get_enrollable_face(&enroll_data, config).await;
        let details = match enroll_data.details {
            Some(details) => details,
            None => {
                return Err(FRError::with_code(
                    1011,
                    "Enrollment details were not provided or could not be resolved",
                ));
            }
        };

        let mut id_req = match ident_req {
            Ok(id_req) => id_req,
            Err(precheck_err) => {
                self.log_enroll_err("create_enrollment", &precheck_err, &details).await;
                let value = json!({
                    "precheck": precheck_err.details.clone(),
                    "enrollment": details,
                });
                return Err(FRError::with_details(precheck_err.code, &precheck_err.message, value));
            }
        };

        id_req.external_ids = ext_id.clone().into_iter().collect();
        let ident_res = self.ident_api.create_identities(id_req).await;

        let ident = match ident_res {
            Ok(idents) if !idents.identities.is_empty() => {
                idents.identities.into_iter().next().ok_or_else(|| {
                    FRError::with_code(
                        1021,
                        "Enrollment failed. No identity was returned from paravision",
                    )
                })?
            }
            Ok(_) => {
                return Err(FRError::with_code(
                    1021,
                    "Enrollment failed. No identity was returned from paravision",
                ));
            }
            Err(e) => {
                let fr_err = FRError::from(e);
                self.log_enroll_err("create_enrollment", &fr_err, &details).await;
                return Err(fr_err);
            }
        };

        let fr_id = ident.id;
        let eid_str = ext_id.unwrap_or_default();
        let eid_num = eid_str.parse::<u64>().unwrap_or(0);
        Ok(EnrollmentCreateResult { fr_id, ext_id: eid_num, ext_id_str: eid_str })
    }

    async fn delete_enrollment(&self, face_id: &str) -> FRResult<EnrollmentDeleteResult> {
        let del_req = delete_identity_request(face_id);

        let res = self.ident_api.delete_identities(del_req).await;

        match res {
            Ok(delete_response) if delete_response.rows_affected > 0 => {
                Ok(EnrollmentDeleteResult { fr_id: face_id.to_string() })
            }
            Ok(_) => {
                let details = json!({ "fr_id": face_id });
                let fr_err = FRError::with_details(
                    1090,
                    "pv returned no rows for delete request",
                    details.clone(),
                );
                self.log_delete_err("delete_enrollment", &fr_err, Some(details)).await;
                Err(fr_err)
            }
            Err(e) => {
                let details = json!({ "fr_id": face_id });
                let fr_err = FRError::with_details(e.code, &e.message, details.clone());
                self.log_delete_err("delete_enrollment", &fr_err, Some(details)).await;
                Err(fr_err)
            }
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<EnrollmentMetadataRecord> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
            r#"
            select
                (select count(*)::bigint from eyefr.profiles) as profiles_total,
                (select count(*)::bigint from eyefr.profiles where fr_id is not null and fr_id <> '') as profiles_with_fr_id,
                (select count(*)::bigint from eyefr.images) as images_total,
                (select count(*)::bigint from eyefr.registration_errors) as registration_errors_total,
                (select count(*)::bigint from logs.enrollment) as enrollment_logs_total
            "#,
        )
        .fetch_one(&self.db)
        .await?;

        Ok(EnrollmentMetadataRecord {
            profiles_total: row.0,
            profiles_with_fr_id: row.1,
            images_total: row.2,
            registration_errors_total: row.3,
            enrollment_logs_total: row.4,
        })
    }

    async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>> {
        let rows = sqlx::query_as::<_, EnrollmentRosterRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            order by last_name asc nulls last, first_name asc nulls last, ext_id asc
            limit 1000
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Self::profile_to_enrollment_item).collect())
    }

    async fn reset_enrollments(&self) -> FRResult<ResetEnrollmentsBackendResult> {
        let list_req = list_identities_request(100000);
        let identities = self.ident_api.get_identities(list_req).await?.identities;

        let deleted_count = identities.len();
        for identity in identities {
            let delete_req = delete_identity_request(&identity.id);
            if let Err(err) = self.ident_api.delete_identities(delete_req).await {
                warn!("reset_enrollments failed deleting {}: {}", identity.id, err);
            }
        }

        info!("Enrollments deleted from pv: {}", deleted_count);

        Ok(ResetEnrollmentsBackendResult {
            msg: "All PV enrollments have been deleted. System reset.".to_string(),
        })
    }

    async fn detect_face(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        let process_req = if liveness_check {
            liveness_process_full_image_request(image)
        } else {
            default_process_full_image_request(image, true)
        };

        let img_resp = self.proc_api.process_full_image(process_req).await?;
        let faces = img_resp.faces.into_iter().map(Face::from).collect();

        Ok(faces)
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let process_req = default_process_full_image_request(image, true);
        let img_resp = self.proc_api.process_full_image(process_req).await?;

        let Some((faces, lookup_req)) = lookup_candidates_from_processed(img_resp, config.top_n)
        else {
            info!("recognize found no matches");
            return Ok(vec![]);
        };

        let lookup_response = self.ident_api.lookup(lookup_req).await?;
        let lookup_identities = lookup_response.lookup_identities;

        if lookup_identities.is_empty() {
            info!("recognize found no lookup identity sets");
            return Ok(vec![]);
        }

        if lookup_identities.len() != faces.len() {
            warn!(
                "recognize face/lookup count mismatch: faces={}, lookup_sets={}",
                faces.len(),
                lookup_identities.len()
            );
        }

        Ok(self.to_fr_identities(&faces, &lookup_identities, config.min_match))
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<AddFaceResult> {
        let process_req = default_process_full_image_request(image, true);
        let processed = self.proc_api.process_full_image(process_req).await?;

        let add_req = add_faces_request_from_processed(processed, fr_id.to_string(), 0.0);
        let face_resp = self.ident_api.add_faces(add_req).await?;

        Ok(to_add_face_result(face_resp))
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<DeleteFaceResult> {
        let delete_req = delete_faces_request(fr_id, face_id);
        let res = self.ident_api.delete_faces(delete_req).await?;

        Ok(DeleteFaceResult { rows_affected: res.rows_affected })
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<GetFaceInfoResult> {
        let req = get_faces_request(fr_id);
        let res = self.ident_api.get_faces(req).await?;
        Ok(to_get_face_info_result(res))
    }

    async fn get_enrollments_by_last_name(
        &self,
        name: &str,
    ) -> FRResult<Vec<EnrollmentRosterItem>> {
        let like_pattern = format!("{}%", name.trim());
        let rows = sqlx::query_as::<_, EnrollmentRosterRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where last_name ilike $1
            order by last_name asc, first_name asc, ext_id asc
            limit 100
            "#,
        )
        .bind(like_pattern)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Self::profile_to_enrollment_item).collect())
    }

    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.log_identification_db(fr_identity, extra, location).await
    }
}
