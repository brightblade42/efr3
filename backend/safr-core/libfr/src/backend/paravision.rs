use super::{FRBackend, FRResult, MatchConfig};
use crate::{utils, EnrollData, EnrollDetails, FRError};
use crate::{FRIdentity, Face, PossibleMatch};
use bytes::Bytes;
use libpv::identity_grpc::PVIdentityGrpcApi;
use libpv::proc_grpc::PVProcGrpcApi;
use libpv::types::{
    AddFaceRequest, CreateIdentitiesRequest, DeleteFaceRequest, DeleteIdentitiesRequest, Embedding,
    GetFacesRequest, LookupRequest, LookupResponse,
};
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
    ) -> FRResult<CreateIdentitiesRequest> {
        let image = enroll_data.image.clone().ok_or_else(|| {
            FRError::with_code(
                1010,
                "An image is required for enrollment but was not found",
            )
        })?;

        let img_resp = self.proc_api.process_image(image, None, true).await?;

        if img_resp.most_prominent_face_idx.is_none() {
            return Err(FRError::with_code(
                1080,
                "enrollment image must be set to use most prominent face with was not set. ",
            ));
        }

        let p_idx = img_resp.most_prominent_face_idx.unwrap_or(0) as usize;
        let faces = img_resp.faces.as_ref().ok_or_else(|| {
            FRError::with_code(1081, "enrollment image processing returned no faces")
        })?;
        let prom_face = faces.get(p_idx).ok_or_else(|| {
            FRError::with_code(1082, "most prominent face index is out of bounds")
        })?;
        let emb = prom_face.embedding.as_ref().ok_or_else(|| {
            FRError::with_code(1083, "most prominent face did not include an embedding")
        })?;

        let embedding = Embedding {
            embedding: emb.to_vec(),
        };

        let resp = self.ident_api.lookup_single(embedding).await?;
        let ident = resp.lookup_identities.first();

        match ident {
            Some(id) => {
                let duplicate = id
                    .matches
                    .iter()
                    .find(|ic| ic.score >= config.min_dupe_match);

                if let Some(confidence_match) = duplicate {
                    let details = json!({
                        "fr_id": confidence_match.identity.id,
                        "created_at": confidence_match.identity.created_at,
                    });

                    Err(FRError::with_details(
                        1020,
                        &format!("Duplicate: {} match", confidence_match.score),
                        details,
                    ))
                } else {
                    Ok(img_resp.into())
                }
            }
            None => Ok(img_resp.into()),
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
            error!(
                "#GREAT SCOTT! The database write failed for log_identification! {:?}",
                err
            );
            return Err(FRError::with_code(
                2021,
                "#GREAT SCOTT! The database write failed for log_identification!",
            ));
        }

        Ok(())
    }

    fn to_fr_identities(&self, lookups: &[LookupResponse], min_match: f32) -> Vec<FRIdentity> {
        lookups
            .iter()
            .map(|item| {
                let mut pms: Vec<PossibleMatch> = item
                    .identities
                    .lookup_identities
                    .iter()
                    .flat_map(|item| &item.matches)
                    .map(|ic| {
                        let n_conf = utils::roundf32(ic.score, 5);
                        let mut pm = PossibleMatch::new(ic.identity.id.clone(), n_conf);
                        pm.ext_id = ic.identity.external_id.clone().unwrap_or_default();
                        pm
                    })
                    .collect();

                if pms.len() > 1 {
                    pms.sort_by(|a, b| {
                        a.score
                            .partial_cmp(&b.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    pms.reverse();
                }

                FRIdentity {
                    face: Face::from(&item.face),
                    possible_matches: pms,
                }
            })
            .filter(|fr| {
                fr.possible_matches
                    .first()
                    .is_some_and(|pm| pm.score >= min_match)
            })
            .collect()
    }

    fn profile_to_enrollment_item(row: EnrollmentRosterRow) -> Value {
        let details = row.raw_data.unwrap_or_else(|| {
            json!({
                "first_name": row.first_name,
                "last_name": row.last_name,
                "middle_name": row.middle_name,
                "img_url": row.img_url,
            })
        });

        json!({
            "fr_id": row.fr_id,
            "ext_id": row.ext_id.parse::<u64>().unwrap_or(0),
            "ext_id_str": row.ext_id,
            "details": details,
        })
    }
}

impl FRBackend for PVBackend {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<String>,
    ) -> FRResult<Value> {
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
            Err(dup_err) => {
                self.log_enroll_err("create_enrollment", &dup_err, &details)
                    .await;
                let value = serde_json::to_value(&details).unwrap_or_else(|_| json!({}));
                return Err(FRError::with_details(1020, &dup_err.message, value));
            }
        };

        id_req.threshold = config.min_dupe_match;
        id_req.external_ids = ext_id.clone().map(|id| vec![id]);
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
                self.log_enroll_err("create_enrollment", &fr_err, &details)
                    .await;
                return Err(fr_err);
            }
        };

        let fr_id = ident.id;
        let eid_str = ext_id.unwrap_or_default();
        let eid_num = eid_str.parse::<u64>().unwrap_or(0);
        Ok(json!({"fr_id": fr_id, "ext_id": eid_num, "ext_id_str": eid_str}))
    }

    async fn delete_enrollment(&self, face_id: &str) -> FRResult<Value> {
        let del_req = DeleteIdentitiesRequest::from(face_id);

        let res = self
            .ident_api
            .delete_identities(Some(del_req))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| FRError::with_code(1090, "pv returned no results for delete."))?;

        match res {
            Ok(del_id) => Ok(json!({ "fr_id": del_id })),
            Err(e) => {
                let details = json!({ "fr_id": face_id });
                let fr_err = FRError::with_details(e.code, &e.message, details.clone());
                self.log_delete_err("delete_enrollment", &fr_err, Some(details))
                    .await;
                Err(fr_err)
            }
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<Value> {
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

        Ok(json!({
            "profiles_total": row.0,
            "profiles_with_fr_id": row.1,
            "images_total": row.2,
            "registration_errors_total": row.3,
            "enrollment_logs_total": row.4,
        }))
    }

    async fn get_enrollment_roster(&self) -> FRResult<Value> {
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

        Ok(Value::Array(
            rows.into_iter()
                .map(Self::profile_to_enrollment_item)
                .collect(),
        ))
    }

    async fn reset_enrollments(&self) -> FRResult<Value> {
        let res = self.ident_api.delete_identities(None).await?;

        info!("Enrollments deleted from pv: {}", res.len());

        Ok(json!({
            "msg" : "All PV enrollments have been deleted. System reset."
        }))
    }

    async fn detect_face(&self, image: Bytes, liveness_check: bool) -> FRResult<Value> {
        let img_resp = if liveness_check {
            self.proc_api.process_image_liveness(image).await?
        } else {
            self.proc_api.process_image(image, None, true).await?
        };

        let faces = match img_resp.faces {
            Some(faces) => faces.into_iter().map(Face::from).collect(),
            None => vec![],
        };

        Ok(serde_json::to_value(faces)?)
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let img_resp = self.proc_api.process_image(image, None, true).await?;

        let mut lookup_req = LookupRequest::from(img_resp);
        lookup_req.limit = config.top_n;

        let lookup_vec = self.ident_api.lookup(lookup_req).await?;

        if lookup_vec.is_empty() {
            info!("recognize found no matches");
            return Ok(vec![]);
        }

        let mut lookups: Vec<LookupResponse> = Vec::with_capacity(lookup_vec.len());
        for lookup in lookup_vec {
            match lookup {
                Ok(item) => lookups.push(item),
                Err(err) => warn!("recognize lookup item failed: {}", err),
            }
        }

        if lookups.is_empty() {
            warn!("recognize: all lookup requests failed");
            return Ok(vec![]);
        }

        Ok(self.to_fr_identities(&lookups, config.min_match))
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<Value> {
        let img_res = self.proc_api.process_image(image, None, true).await?;

        let mut af_req = AddFaceRequest::from(img_res);
        af_req.identity_id = fr_id.to_string();
        af_req.threshold = 0.0;

        let face_resp = self.ident_api.add_face(af_req).await?;
        Ok(serde_json::to_value(face_resp)?)
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value> {
        let del_req = DeleteFaceRequest {
            fr_id: fr_id.to_string(),
            face_id: face_id.to_string(),
        };

        let res = self.ident_api.delete_face(&del_req).await?;
        Ok(serde_json::to_value(res)?)
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<Value> {
        let req = GetFacesRequest {
            fr_id: fr_id.to_string(),
        };

        let res = self.ident_api.get_faces(req).await?;
        Ok(serde_json::to_value(res)?)
    }

    async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
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

        Ok(rows
            .into_iter()
            .map(Self::profile_to_enrollment_item)
            .collect())
    }

    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.log_identification_db(fr_identity, extra, location)
            .await
    }
}
