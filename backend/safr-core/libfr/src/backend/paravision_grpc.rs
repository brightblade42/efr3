use super::{paravision::PVBackend, FRBackend, FRResult, MatchConfig};
use crate::{EnrollData, FRError};
use crate::{FRIdentity, Face};
use bytes::Bytes;
use libpv::identity_grpc::PVIdentityGrpcApi;
use libpv::proc_grpc::PVProcGrpcApi;
use libpv::types::{
    AddFaceRequest, CreateIdentitiesRequest, DeleteFaceRequest, DeleteIdentitiesRequest, Embedding,
    GetFacesRequest, LookupRequest, LookupResponse,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{info, warn};

#[derive(Clone)]
pub struct PVGrpcBackend {
    legacy: PVBackend,
    proc_api: PVProcGrpcApi,
    ident_api: PVIdentityGrpcApi,
}

impl PVGrpcBackend {
    pub fn new(proc_url: String, ident_url: String, db: PgPool) -> Self {
        let legacy = PVBackend::new(proc_url.clone(), ident_url.clone(), db);
        let proc_api = PVProcGrpcApi::new(proc_url);
        let ident_api = PVIdentityGrpcApi::new(ident_url);

        Self {
            legacy,
            proc_api,
            ident_api,
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
                    .identity_confidences
                    .iter()
                    .find(|ic| ic.confidence >= config.min_dupe_match);

                if let Some(confidence_match) = duplicate {
                    let details = json!({
                        "fr_id": confidence_match.identity.id,
                        "created_at": confidence_match.identity.created_at,
                    });

                    Err(FRError::with_details(
                        1020,
                        &format!("Duplicate: {} match", confidence_match.confidence),
                        details,
                    ))
                } else {
                    Ok(img_resp.into())
                }
            }
            None => Ok(img_resp.into()),
        }
    }
}

impl FRBackend for PVGrpcBackend {
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
                self.legacy
                    .log_enroll_err("create_enrollment", &dup_err, &details)
                    .await;
                let value = serde_json::to_value(&details).unwrap_or_else(|_| json!({}));
                return Err(FRError::with_details(1020, &dup_err.message, value));
            }
        };

        id_req.confidence = config.min_dupe_match;
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
                self.legacy
                    .log_enroll_err("create_enrollment", &fr_err, &details)
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
                self.legacy
                    .log_delete_err("delete_enrollment", &fr_err, Some(details))
                    .await;
                Err(fr_err)
            }
        }
    }

    async fn get_enrollment_metadata(&self) -> FRResult<Value> {
        self.legacy.get_enrollment_metadata().await
    }

    async fn get_enrollment_roster(&self) -> FRResult<Value> {
        self.legacy.get_enrollment_roster().await
    }

    async fn reset_enrollments(&self) -> FRResult<Value> {
        let res = self.ident_api.delete_identities(None).await?;

        info!("Enrollments deleted from pv: {}", res.len());

        Ok(json!({
            "msg" : "All PV enrollments have been deleted. System reset."
        }))
    }

    async fn detect_face(&self, image: Bytes, spoof_check: bool) -> FRResult<Value> {
        let img_resp = if spoof_check {
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

        let fr_idents = self
            .legacy
            .to_fr_identities(&lookups, config.min_match, true)
            .await?;
        Ok(fr_idents)
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<Value> {
        let img_res = self.proc_api.process_image(image, None, true).await?;

        let mut af_req = AddFaceRequest::from(img_res);
        af_req.identity_id = fr_id.to_string();
        af_req.confidence_threshold = 0;

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
        self.legacy.get_enrollments_by_last_name(name).await
    }

    async fn log_identity(
        &self,
        fr_identity: &FRIdentity,
        extra: Option<&Value>,
        location: &str,
    ) -> FRResult<()> {
        self.legacy.log_identity(fr_identity, extra, location).await
    }
}
