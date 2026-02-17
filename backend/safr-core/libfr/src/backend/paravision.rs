use super::{FRBackend, FRResult, MatchConfig};
use crate::{utils, EnrollData, EnrollDetails, FRError};
use crate::{FRIdentity, Face, MinDetails, PossibleMatch};
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use libpv::types::{
    AddFaceRequest, CreateIdentitiesRequest, DeleteFaceRequest, DeleteIdentitiesRequest, Embedding,
    GetFacesRequest, Identity, LookupRequest, LookupResponse, ProcessFullImageRequest,
};
use libpv::PVApi;
use serde_json::{json, Value};

use sqlx::PgPool;

use std::collections::HashMap;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct PVBackend {
    api: PVApi,
    db: PgPool,
}

pub struct FRIDPair {
    pub image_id: String,
    pub face_id: String,
}

#[derive(sqlx::FromRow)]
struct EnrollmentRow {
    fr_id: String,
    ext_id: String,
    summary: Value,
}

impl EnrollmentRow {
    fn into_min_details(self) -> MinDetails {
        MinDetails {
            ext_id: parse_ext_id_or_default(&self.fr_id, &self.ext_id),
            fr_id: self.fr_id,
            details: self.summary,
        }
    }
}

fn parse_ext_id_or_default(fr_id: &str, ext_id: &str) -> u64 {
    match ext_id.parse::<u64>() {
        Ok(id) => id,
        Err(err) => {
            warn!(
                "failed to parse ext_id '{}' for fr_id '{}': {}; defaulting to 0",
                ext_id, fr_id, err
            );
            0
        }
    }
}

impl PVBackend {
    ///creates the backend using an existing database pool
    pub fn new(proc_url: String, ident_url: String, db: PgPool) -> Self {
        Self {
            api: PVApi::new(proc_url, ident_url),
            db,
        }
    }

    ///checks if there is a close enough match to determine if a person is already enrolled,
    //perhaps option, to add as secondary face or reject. for now, reject
    async fn get_enrollable_face(
        &self,
        enroll_data: &EnrollData,
        config: MatchConfig,
    ) -> FRResult<CreateIdentitiesRequest> {
        //let r = self.api.lookup(req) //

        let img_resp = self.api.process_image(enroll_data).await?;

        //one face per image please.
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

        //cool
        let embedding = Embedding {
            embedding: emb.to_vec(),
        };

        let resp = self.api.lookup_single(embedding).await?;

        let ident = resp.lookup_identities.first();

        debug!("{:?}", &ident);

        let is_duped = match ident {
            Some(id) => {
                let r = id
                    .identity_confidences
                    .iter()
                    .find(|ic| ic.confidence >= config.min_dupe_match);

                if let Some(cc) = r {
                    //Err(FRError::with_code(1020, &format!("Duplicate: {} match", cc.confidence)))
                    let det = json!({
                        "fr_id": cc.identity.id,
                        "created_at": cc.identity.created_at
                    });

                    Err(FRError::with_details(
                        1020,
                        &format!("Duplicate: {} match", cc.confidence),
                        det,
                    ))
                } else {
                    Ok(img_resp.into())
                }
            }
            None => Ok(img_resp.into()),
        };

        is_duped
    }

    pub(crate) async fn enroll_db(
        &self,
        fr_data: &Identity,
        details: &EnrollDetails,
        ext_id: Option<u64>,
    ) -> FRResult<()> {
        debug!("The data we will be shoving into our db");
        debug!("{:?}", fr_data);

        let eid = ext_id.unwrap_or(0);
        let id = &fr_data.id;

        // //TODO: can't sqlx do this for us?
        let details_val = serde_json::to_value(details)?;
        let fr_data_val = serde_json::to_value(&fr_data)?;

        // //NOTE: image_id and face_id. it's not entirely clear which is the true identifier
        // //TODO: if this fails, then what?

        let res = sqlx::query(
            r" INSERT into paravision.enrollment (fr_id,fr_data,summary, ext_id) VALUES ($1,$2,$3,$4) ",
        )
        .bind(id)
        .bind(fr_data_val)
        .bind(details_val)
        .bind(eid.to_string())
        .execute(&self.db)
        .await?;

        if res.rows_affected() != 1 {
            return Err(FRError::with_details(
                1012,
                "Enrollment insert did not affect exactly one row",
                json!({
                    "fr_id": id,
                    "rows_affected": res.rows_affected(),
                }),
            ));
        }

        Ok(())
    }

    ///delete an enrollment by the id given from the fr api.
    pub(crate) async fn delete_enrollment_db(&self, fr_id: &str) -> FRResult<u64> {
        let res = sqlx::query("DELETE from paravision.enrollment where fr_id = $1")
            .bind(fr_id)
            .execute(&self.db)
            .await?;

        let rows = res.rows_affected();
        if rows == 0 {
            warn!(
                "delete_enrollment_db removed no rows for fr_id '{}'. database and pv may be out of sync",
                fr_id
            );
        }

        info!("delete_enrollment_db: {:?} ", res);

        Ok(rows)
    }

    pub(crate) async fn delete_all_enrollments_db(&self) -> FRResult<u64> {
        let res = sqlx::query("Delete from paravision.enrollment")
            .execute(&self.db)
            .await?;
        let rows = res.rows_affected();
        info!("deleted {} local enrollment rows", rows);
        Ok(rows)
    }

    ///if there's an error during enrollment, we log it.
    ///depending on the kind of error we may be able to retry later by pulling
    ///from the log table.
    pub(crate) async fn log_enroll_err(&self, action: &str, e: &FRError, details: &EnrollDetails) {
        error!("Enrollment error: {}", e.message); //what if the log goes wrong?
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
            r" INSERT into paravision.enrollment_log (action,error,message,details) VALUES ($1,$2,$3,$4) ")
            .bind(action)
            .bind(e_val)
            .bind(&e.message)
            .bind(details_val)
            .execute(&self.db).await;

        //the log itself failed, if this happens we're in all kinds of trouble
        if res.is_err() {
            error!("#GREAT SCOTT! The database write failed! Abandon all hope. we should panic.");
        }
    }

    //log the time and place of an identification with optional data
    async fn log_identification_db(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        debug!("log_identification_db: logging the time and place a person was identified.");

        let pm_res = fr_identity.possible_matches.first();
        let pm = match pm_res {
            None => {
                return Err(FRError::with_code(
                    2020,
                    "could not log positive identification. identity has no possible matches",
                ))
            }
            Some(pm) => pm,
        };

        let confidence = pm.confidence;

        let pm_val = serde_json::to_value(pm)?;

        let res = sqlx::query(r"Insert into public.fr_log (pmatch, extra, location, confidence) VALUES ($1, $2, $3, $4)")
            .bind(pm_val)
            .bind(extra)
            .bind(location)
            .bind(confidence)
            .execute(&self.db).await;

        if let Err(err) = res {
            error!("#GREAT SCOTT! The database write failed for log_identification! Abandon all hope. {:?}", err);
            return Err(FRError::with_code(
                2021,
                "#GREAT SCOTT! The database write failed for log_identification! Abandon all hope",
            ));
        };

        Ok(())
    }

    pub(crate) async fn log_delete_err(&self, action: &str, e: &FRError, details: Option<Value>) {
        error!("Delete Enrollment error: {}", e.message); //what if the log goes wrong?
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
            r" INSERT into paravision.enrollment_log (action,error,message,details) VALUES ($1,$2,$3,$4) ")
            .bind(action)
            .bind(e_val)
            .bind(&e.message)
            .bind(details_val)
            .execute(&self.db).await;

        //the log itself failed, if this happens we're in all kinds of trouble
        if let Err(_lg_err) = res {
            error!("#GREAT SCOTT! The database write failed! Abandon all hope. we should panic.");
        }
    }

    ///convert the data from paravision specific to our own format and add some
    ///basic detail information that was provided during enrollment.
    pub(crate) async fn to_fr_identities(
        &self,
        lookups: &[LookupResponse],
        min_match: f32,
        _verbose: bool,
    ) -> Result<Vec<FRIdentity>, FRError> {
        //FIXME: lookup_identities is an array that contains an array of
        // identity_confidences. This doesn't seem right. should be a single
        // array repr all possible matches for a face.
        // we use a flat_map to hoist up the inner vec but this is just
        // working around a poor structure

        //this is crazy time yo.
        //transform to friendlier types, do a sort, filter out low confidences.
        let mut fr_idents: Vec<FRIdentity> = lookups
            .iter()
            .map(|item| {
                let mut pms: Vec<PossibleMatch> = item
                    .identities
                    .lookup_identities
                    .iter()
                    .flat_map(|item| &item.identity_confidences)
                    .map(|ic| {
                        let n_conf = utils::roundf32(ic.confidence, 5);
                        PossibleMatch::new(ic.identity.id.clone(), n_conf)
                    })
                    .collect();

                if pms.len() > 1 {
                    pms.sort_by(|a, b| {
                        a.confidence
                            .partial_cmp(&b.confidence)
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
                    .is_some_and(|pm| pm.confidence >= min_match)
            })
            .collect();

        let mut fr_ids: Vec<String> = fr_idents
            .iter()
            .flat_map(|ident| ident.possible_matches.iter().map(|pm| pm.fr_id.clone()))
            .collect();

        fr_ids.sort();
        fr_ids.dedup();

        let details_by_fr_id = self.get_details_by_fr_ids(&fr_ids).await?;

        for ident in &mut fr_idents {
            for pm in &mut ident.possible_matches {
                if let Some(loc_det) = details_by_fr_id.get(&pm.fr_id) {
                    pm.ext_id = loc_det.ext_id;
                    pm.details = Some(loc_det.details.clone())
                } else {
                    error!(
                        "pv identity {} exists but not in enrollment db. db out of sync",
                        &pm.fr_id
                    );
                }
            }
        }

        Ok(fr_idents) //let's go boys!
    }

    async fn get_details_by_fr_ids(
        &self,
        fr_ids: &[String],
    ) -> FRResult<HashMap<String, MinDetails>> {
        if fr_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows: Vec<EnrollmentRow> = sqlx::query_as(
            r#"SELECT fr_id, ext_id, summary FROM paravision.enrollment WHERE fr_id = ANY($1)"#,
        )
        .bind(fr_ids)
        .fetch_all(&self.db)
        .await?;

        let details_by_fr_id = rows
            .into_iter()
            .map(EnrollmentRow::into_min_details)
            .map(|min| (min.fr_id.clone(), min))
            .collect();

        Ok(details_by_fr_id)
    }

    async fn get_details_from_name(&self, name: &str) -> FRResult<Vec<MinDetails>> {
        let like_pattern = format!("{}%", name);
        let rows: Vec<EnrollmentRow> = sqlx::query_as(
            r#"SELECT fr_id, ext_id, summary FROM paravision.enrollment WHERE summary->>'last_name' LIKE $1"#,
        )
        .bind(like_pattern)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(EnrollmentRow::into_min_details)
            .collect())
    }
}

impl FRBackend for PVBackend {
    async fn create_enrollment(
        &self,
        enroll_data: EnrollData,
        config: MatchConfig,
        ext_id: Option<u64>,
    ) -> FRResult<Value> {
        //at this point, we must have acquired an image from some source. If we don't, we can't enroll
        if enroll_data.image.is_none() {
            return Err(FRError::with_code(
                1010,
                "An image is required for enrollment but was not found",
            ));
        }

        let ident_req = self.get_enrollable_face(&enroll_data, config).await;
        //dupe check here, we've validated the presence of an image, now we must check existence.
        //TODO: these are the minimum details, we want to add out full details as well yes | no
        let details = match enroll_data.details {
            Some(details) => details,
            None => {
                return Err(FRError::with_code(
                    1011,
                    "Enrollment details were not provided or could not be resolved",
                ));
            }
        }; //details could be None if remote call fails or returns nothing.

        let mut id_req = match ident_req {
            Ok(id_req) => id_req,
            Err(dup_err) => {
                error!("Received PV dupes. log it. ");
                self.log_enroll_err("create_enrollment", &dup_err, &details)
                    .await;
                let v = serde_json::to_value(&details).unwrap_or_else(|_| json!({}));
                return Err(FRError::with_details(1020, &dup_err.message, v));
            }
        };

        //if we got here, we are cleared for performing the enrollment.
        id_req.confidence = config.min_dupe_match; //basically a double dupe check.
        let ident_res = self.api.create_identities(id_req).await; //fr template create

        let ident = match ident_res {
            Ok(idents) if !idents.identities.is_empty() => {
                idents.identities.into_iter().next().ok_or_else(|| {
                    FRError::with_code(
                        1021,
                        "Enrollment failed. No identity was returned from paravision",
                    )
                })?
            }
            Ok(_idents) => {
                return Err(FRError::with_code(
                    1021,
                    "Enrollment failed. No identity was returned from paravision",
                ))
            }
            Err(e) => {
                //log enrollment error and return
                let fr_err = FRError::from(e);
                self.log_enroll_err("create_enrollment", &fr_err, &details)
                    .await;
                return Err(fr_err);
            }
        };

        //NOTE: We have passed the gauntlet of possible errors! Rejoice!

        //pass in optional external details
        //NOTE: if enroll_db fails, this operation is partial in PV and should fail fast here.
        if let Err(db_err) = self.enroll_db(&ident, &details, ext_id).await {
            self.log_enroll_err("enroll_db", &db_err, &details).await;

            let rollback_req = DeleteIdentitiesRequest::from(ident.id.as_str());
            match self.api.delete_identities(Some(rollback_req)).await {
                Ok(results) => {
                    if !results.into_iter().any(|res| res.is_ok()) {
                        error!(
                            "failed local enrollment write and no pv rollback results succeeded for fr_id {}",
                            ident.id
                        );
                    }
                }
                Err(rollback_err) => {
                    error!(
                        "failed local enrollment write and pv rollback for fr_id {}: {}",
                        ident.id, rollback_err
                    );
                }
            }

            return Err(db_err);
        }

        let fr_id = ident.id;
        let eid = ext_id.unwrap_or(0);

        //TODO: too much json. start thinking about structs
        Ok(json!({"fr_id": fr_id, "ext_id": eid}))
    }

    async fn delete_enrollment(&self, face_id: &str) -> FRResult<Value> {
        let del_req = DeleteIdentitiesRequest::from(face_id);

        let res = self
            .api
            .delete_identities(Some(del_req))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| FRError::with_code(1090, "pv returned no results for delete."))?;

        match res {
            Ok(del_id) => {
                let deleted_rows = self.delete_enrollment_db(&del_id).await?;
                if deleted_rows == 0 {
                    warn!(
                        "delete_enrollment succeeded in pv but found no local row for fr_id {}",
                        del_id
                    );
                }

                Ok(json!({ "fr_id": del_id })) //we'll want our own data structure, maybe that's for one level up
            }
            Err(e) => {
                let details = json!({ "fr_id": face_id });
                let fr_err = FRError::with_details(e.code, &e.message, details.clone());
                self.log_delete_err("delete_enrollment", &fr_err, Some(details))
                    .await;
                Err(fr_err)
            }
        }
    }
    //delete an enrollment for a singel face
    async fn get_enrollment_metadata(&self) -> FRResult<Value> {
        todo!()
    }
    async fn get_enrollment_roster(&self) -> FRResult<Value> {
        todo!()
    }

    async fn reset_enrollments(&self) -> FRResult<Value> {
        //passsing none means delete all.. which is weird.
        let res = self.api.delete_identities(None).await?; //.into_iter().next();
        let db_deleted = self.delete_all_enrollments_db().await?;

        info!(
            "Enrollments deleted from pv: {} local rows deleted: {}",
            res.len(),
            db_deleted
        );

        Ok(json!({
            "msg" : "All PV enrollments have been deleted. System reset."
        }))
    }

    ///This returns attributes about a face in an image
    ///this is different than an identity that a face represents which requires recognition
    async fn detect_face(&self, image: Bytes, spoof_check: bool) -> FRResult<Value> {
        if spoof_check {
            warn!("spoof check was requested but no avail in paravision v6");
        }

        let img_req = ProcessFullImageRequest {
            image: general_purpose::STANDARD.encode(image),
            ..ProcessFullImageRequest::default()
        };

        let img_resp = self.api.process_image(img_req).await?;

        let faces = match img_resp.faces {
            Some(faces) => faces.into_iter().map(Face::from).collect(),
            None => vec![],
        };

        Ok(serde_json::to_value(faces)?)
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        let img_req = ProcessFullImageRequest {
            image: general_purpose::STANDARD.encode(image),
            ..ProcessFullImageRequest::default()
        };

        let img_resp = self.api.process_image(img_req).await?;

        let mut lookup_req = LookupRequest::from(img_resp);

        lookup_req.limit = config.top_n; //5; //top 5 but this should be an option

        let lookup_vec = self.api.lookup(lookup_req).await?;

        //TODO: account for min match

        //nuthing, Lebowski. Nothing!
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

        let fr_idents = self
            .to_fr_identities(&lookups, config.min_match, true)
            .await?;

        Ok(fr_idents)
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<Value> {
        let img_req = ProcessFullImageRequest {
            image: general_purpose::STANDARD.encode(image),
            ..ProcessFullImageRequest::default()
        };
        let img_res = self.api.process_image(img_req).await?;

        let mut af_req = AddFaceRequest::from(img_res);
        af_req.identity_id = fr_id.to_string();
        af_req.confidence_threshold = 0;

        let face_resp = self.api.add_face(af_req).await?;

        Ok(serde_json::to_value(face_resp)?)
    }

    async fn delete_face(&self, fr_id: &str, face_id: &str) -> FRResult<Value> {
        let del_req = DeleteFaceRequest {
            fr_id: fr_id.to_string(),
            face_id: face_id.to_string(),
        };

        let res = self.api.delete_face(&del_req).await?;
        Ok(serde_json::to_value(res)?)
    }

    async fn get_face_info(&self, fr_id: &str) -> FRResult<Value> {
        let req = GetFacesRequest {
            fr_id: fr_id.to_string(),
        };

        let res = self.api.get_faces(req).await?;
        Ok(serde_json::to_value(res)?)
    }

    async fn get_enrollments_by_last_name(&self, name: &str) -> FRResult<Vec<Value>> {
        let res = self.get_details_from_name(name).await?;

        Ok(res
            .into_iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.log_identification_db(fr_identity, extra, location)
            .await
        //Ok(())
    }
}
