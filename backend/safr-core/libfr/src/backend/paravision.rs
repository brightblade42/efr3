use super::{
    pvtypes::{
        add_faces_request_from_processed, build_lookup_request, build_process_image_request,
        delete_faces_request, liveness_process_full_image_request, possible_matches_from_lookup,
        DEFAULT_BUCKETS_LIMIT, DEFAULT_SCALING_FACTOR,
    },
    FRBackend, FRResult, MatchConfig, Template,
};
use crate::{
    backend::{pvtypes::timestamp_to_rfc3339, IDSet},
    repo::{EnrollmentMetadataRecord, ProfileRecord},
    EnrolledFaceInfo,
};
use crate::{DeleteFaceResult, EnrollmentRosterItem, FRError, FRIdentity, Face, IDPair};
use bytes::Bytes;
use libpv::identity_grpc::{identity, PVIdentityGrpcApi};
use libpv::proc_grpc::{processor, PVProcGrpcApi};
use serde_json::json;
use sqlx::PgPool;
use tracing::info;

#[derive(Clone)]
pub struct PVBackend {
    proc_api: PVProcGrpcApi,
    ident_api: PVIdentityGrpcApi,
    db: PgPool,
}

impl PVBackend {
    pub fn new(proc_url: String, ident_url: String, db: PgPool) -> Self {
        Self {
            proc_api: PVProcGrpcApi::new(proc_url),
            ident_api: PVIdentityGrpcApi::new(ident_url),
            db,
        }
    }

    // async fn log_identification_db(
    //     &self,
    //     fr_identity: &FRIdentity,
    //     extra: Option<&Value>,
    //     location: &str,
    // ) -> FRResult<()> {
    //     let pm = fr_identity.possible_matches.first().ok_or_else(|| {
    //         FRError::with_code(
    //             2020,
    //             "log_failed_error",
    //             "could not log positive identification. identity has no possible matches",
    //         )
    //     })?;

    //     let confidence = pm.score;
    //     let pm_val = serde_json::to_value(pm)?;

    //     let res = sqlx::query(
    //         r"Insert into logs.matches (pmatch, extra, location, confidence) VALUES ($1, $2, $3, $4)",
    //     )
    //     .bind(pm_val)
    //     .bind(extra)
    //     .bind(location)
    //     .bind(confidence)
    //     .execute(&self.db)
    //     .await;

    //     if let Err(err) = res {
    //         error!("#GREAT SCOTT! The database write failed for log_identification! {:?}", err);
    //         return Err(FRError::with_code(
    //             2021,
    //             "log_failed_error",
    //             "#GREAT SCOTT! The database write failed for log_identification!",
    //         ));
    //     }

    //     Ok(())
    // }

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
                //make sure the first possible match clears the threshold,if the first doesn't , none of the other will either.
                fr_identity
                    .possible_matches
                    .first()
                    .is_some_and(|possible_match| possible_match.score >= min_match)
            })
            .collect()
    }

    fn profile_to_enrollment_item(profile: ProfileRecord) -> EnrollmentRosterItem {
        let details = profile.raw_data.unwrap_or_else(|| {
            json!({
                "first_name": profile.first_name,
                "last_name": profile.last_name,
                "middle_name": profile.middle_name,
                "img_url": profile.img_url,
            })
        });

        EnrollmentRosterItem { fr_id: profile.fr_id, ext_id: profile.ext_id, details }
    }

    fn build_ident_request(
        &self,
        face: &Face,
        dupe_match: f32,
        ext_id: &str,
    ) -> identity::CreateIdentitiesRequest {
        let emb = face.template.clone().unwrap().embedding;

        identity::CreateIdentitiesRequest {
            group_ids: vec![],
            embeddings: vec![identity::Embedding { embedding: emb }],
            threshold: dupe_match,
            model: String::new(),
            qualities: vec![face.quality.unwrap_or(0.0)],
            external_ids: vec![ext_id.to_string()],
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
            options: vec![],
        }
    }
}

impl FRBackend for PVBackend {
    //TODO: maybe kill this
    async fn generate_template(&self, _image: Bytes) -> FRResult<Vec<Template>> {
        Ok(vec![])
    }

    //TODO: maybe kill this
    async fn create_identity(&self, _template: Template, ext_id: &str) -> FRResult<IDSet> {
        Ok(IDSet { fr_id: "abc_123".into(), ext_id: ext_id.into() })
    }

    //not the best name , since it represents 1/2 of the enrollment process
    async fn create_enrollment(
        &self,
        face: &Face,
        config: MatchConfig,
        ext_id: &str,
    ) -> FRResult<IDPair> {
        let id_req = self.build_ident_request(face, config.min_dupe_match, ext_id);

        let ident_res = self.ident_api.create_identities(id_req).await?;

        //NOTE: if create identities returns without error, this seems unlikely to not have an identity
        let fr_id = ident_res
            .identities
            .into_iter()
            .next()
            .ok_or_else(|| FRError::CreateIdentity { ext_id: ext_id.to_string() })?;

        Ok(IDPair { fr_id: fr_id.id, ext_id: ext_id.to_string() })
    }

    //retrieve some basic count. could be better
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

    //TODO: needs work. need a paging strategy
    async fn get_enrollment_roster(&self) -> FRResult<Vec<EnrollmentRosterItem>> {
        let rows = sqlx::query_as::<_, ProfileRecord>(
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

    async fn detect_faces(&self, image: Bytes, liveness_check: bool) -> FRResult<Vec<Face>> {
        let process_req = if liveness_check {
            liveness_process_full_image_request(image)
        } else {
            build_process_image_request(image)
        };

        Ok(self
            .proc_api
            .process_full_image(process_req)
            .await?
            .faces
            .into_iter()
            .map(Face::from)
            .collect())
    }

    async fn recognize(&self, image: Bytes, config: MatchConfig) -> FRResult<Vec<FRIdentity>> {
        //TODO: not sure this is actually correct
        let process_req = build_process_image_request(image);
        let img_resp = self.proc_api.process_full_image(process_req).await?;

        //we don't want liveness data in the recognize results

        let Some((faces, lookup_req)) = build_lookup_request(img_resp, config.top_n) else {
            info!("recognize found no matches");
            return Ok(vec![]);
        };

        let lookup_response = self.ident_api.lookup(lookup_req).await?;
        let lookup_identities = lookup_response.lookup_identities;

        if lookup_identities.is_empty() {
            info!("recognize found no matches");
            return Ok(vec![]);
        }

        //we get empty liveness objects, remove them from results, just noise.
        let mut idents = self.to_fr_identities(&faces, &lookup_identities, config.min_match);
        for ident in &mut idents {
            ident.face.liveness = None;
        }
        Ok(idents)
    }

    async fn add_face(&self, fr_id: &str, image: Bytes) -> FRResult<EnrolledFaceInfo> {
        let process_req = build_process_image_request(image);
        let processed = self.proc_api.process_full_image(process_req).await?;

        let add_req = add_faces_request_from_processed(processed, fr_id.to_string(), 0.0);
        let face_resp = self.ident_api.add_faces(add_req).await?;

        let face = face_resp
            .faces
            .into_iter()
            .next()
            .ok_or_else(|| FRError::AddFace { fr_id: fr_id.to_string() })?;

        Ok(EnrolledFaceInfo {
            face_id: face.id,
            fr_id: face.identity_id,
            created_at: timestamp_to_rfc3339(face.created_at),
            quality: face.quality,
        })
    }

    async fn delete_faces(&self, fr_id: &str, face_ids: Vec<String>) -> FRResult<DeleteFaceResult> {
        let delete_req = delete_faces_request(fr_id, face_ids);
        let res = self.ident_api.delete_faces(delete_req).await?;

        Ok(DeleteFaceResult { rows_affected: res.rows_affected })
    }

    async fn get_enrollments_by_last_name(
        &self,
        name: &str,
    ) -> FRResult<Vec<EnrollmentRosterItem>> {
        let like_pattern = format!("{}%", name.trim());
        let rows = sqlx::query_as::<_, ProfileRecord>(
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
}
