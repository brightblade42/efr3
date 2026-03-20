use axum::{
    extract::{multipart::Multipart, Query, State},
    Json,
};
use bytes::Bytes;
use libfr::{backend::MatchConfig, remote::Remote, SearchBy};
use libfr::{errors::FRError, repo::EnrollmentMetadataRecord};
use libfr::{EnrollData, EnrollDetails, EnrolledFaceInfo, EnrollmentDeleteResult, IDPair};
use libtpass::types::TPassProfile;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{errors::AppError, extractors, AppState, WResult};

pub async fn search_enrollment(
    State(app_state): State<AppState>,
    Json(search_by): Json<SearchEnrollmentBy>,
) -> WResult<Json<Vec<Value>>> {
    let SearchEnrollmentBy::LastName(term) = search_by;

    let res = app_state.fr_service.get_enrollments_by_last_name(&term).await?;
    Ok(Json(res))
}

// FR enrollment flow: image + details are transformed and sent to backend service.
pub async fn create_enrollment(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<IDPair>> {
    let enroll_data = extractors::extract_enroll_data(multipart).await?;

    let res = app_state
        .fr_service
        .create_enrollment(&enroll_data, MatchConfig::from(&app_state.config))
        .await?;
    Ok(Json(res))
}

/// Returns a list of every enrollment in the system. We will want to add paging.
pub async fn get_roster(State(app_state): State<AppState>) -> WResult<Json<Vec<Value>>> {
    let x = app_state.fr_service.get_roster().await?;
    Ok(Json(x))
}

pub async fn delete_enrollment(
    State(app_state): State<AppState>,
    Json(payload): Json<DeleteEnrollmentBy>,
) -> WResult<Json<EnrollmentDeleteResult>> {
    let fr_id = validate_delete(payload)?;
    let res = app_state
        .fr_service
        .delete_enrollment(&fr_id)
        .await
        .inspect_err(|e| error!(target: "enrollment", "{}", e))?;
    info!("deleted enrollment:  fr_id: {}", &res.fr_id);
    Ok(Json(res))
}

/// Deletes all enrollments and resets everything.
pub async fn reset_enrollments(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state.fr_service.reset_enrollments().await?;
    let mut msg = "All enrollments deleted";
    if res == 0 {
        msg = "There were no existing enrollments to delete";
    }
    Ok(Json(json!({
        "msg": msg.to_string(),
        "total": res
    })))
}

pub async fn add_face(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<EnrolledFaceInfo>> {
    let face_req = extractors::extract_add_face_form_data(multipart).await?;

    let res = app_state.fr_service.add_face(&face_req.fr_id, face_req.image.unwrap()).await?;
    Ok(Json(res))
}

pub async fn delete_faces(
    State(app_state): State<AppState>,
    Json(req): Json<DeleteFaceRequest>,
) -> WResult<Json<Value>> {
    if req.fr_id.trim().is_empty() || req.face_ids.is_empty() {
        return Err(AppError::Generic("fr_id and at least one face_id are required".to_string()));
    }

    // 2. Check if any of the actual strings inside the array are just blank spaces
    let has_blank_ids = req.face_ids.iter().any(|id| id.trim().is_empty());
    if has_blank_ids {
        return Err(AppError::Generic(
            "One or more face_ids provided are empty strings".to_string(),
        ));
    }

    let res = app_state.fr_service.delete_faces(&req.fr_id, req.face_ids.clone()).await?;
    Ok(Json(json!({
        "rows_affected": res.rows_affected,
        "fr_id": req.fr_id,
        "face_ids": req.face_ids,
    }
    )))
}

pub async fn get_enrollment_errlog(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let logs = app_state
        .fr_repo
        .get_enrollment_logs(100)
        .await
        .map_err(|e| AppError::Generic(format!("failed to load enrollment logs: {}", e)))?;

    let value = serde_json::to_value(logs)
        .map_err(|e| AppError::Generic(format!("failed to serialize enrollment logs: {}", e)))?;

    Ok(Json(value))
}

/// Gets metadata about the enrollment database.
pub async fn get_enrollment_metadata(
    State(app_state): State<AppState>,
) -> WResult<Json<EnrollmentMetadataRecord>> {
    let res = app_state.fr_service.get_enrollment_metadata().await?;
    Ok(Json(res))
}

//NOTE: old code had multiple possible option but we only want FRID.
// this does that without making a big change and leaves the option for
// adding back more later.
fn validate_delete(del_by: DeleteEnrollmentBy) -> WResult<String> {
    match del_by {
        DeleteEnrollmentBy::FrId(id) if !id.is_empty() => Ok(id),
        DeleteEnrollmentBy::FrId(_) => {
            return Err(AppError::InvalidInput("fr_id is empty".to_string()));
        }
        _ => {
            return Err(AppError::InvalidInput("you must delete by fr_id".to_string()));
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum DeleteEnrollmentBy {
    #[serde(rename = "fr_id")]
    FrId(String),
    #[serde(rename = "ccode")]
    ExtID(u64),
    // Name(String, String),
    // FullName(FullName),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum SearchEnrollmentBy {
    #[serde(rename = "last_name")]
    LastName(String),
}

//TODO: deprecate
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DeleteFaceRequest {
    pub fr_id: String,
    pub face_ids: Vec<String>,
}

//-----    Version 1 backport -------------
//
pub async fn create_enrollment_v1(
    State(app_state): State<AppState>,
    Json(en_cmd): Json<EnrollCommand>,
) -> WResult<Json<EnrollmentResultV1>> {
    let enroll_data = build_enroll_data(&app_state, &en_cmd).await?;

    match app_state
        .fr_service
        .create_enrollment(&enroll_data, MatchConfig::from(&app_state.config))
        .await
    {
        Ok(_) => Ok(Json(EnrollmentResultV1::default())),
        Err(FRError::Duplicate { ext_id, .. }) => {
            let dupe_item = DupeItem { ccode: ext_id, ..Default::default() };

            let en_res = EnrollmentResultV1 {
                dupe_count: 1,
                enroll_count: 0,
                duplicates: vec![dupe_item],
                ..Default::default()
            };
            Ok(Json(en_res))
        }
        Err(e) => {
            error!("v1 enrollment failed:  {}", e);
            Err(AppError::Generic("Enrollment failed for Tpass client".to_string()))
        }
    }
}

//The messiness of V1. validates input and returns EnrollData for create_enrollment
pub async fn build_enroll_data(
    app_state: &AppState,
    en_cmd: &EnrollCommand,
) -> WResult<EnrollData> {
    let ccode = en_cmd
        .candidates
        .first()
        .ok_or_else(|| AppError::Generic("ccode not provided".to_string()))?
        .ccode
        .clone();

    let include_image = true;

    let s_res = app_state
        .tpass_client
        .search_one(SearchBy::ExtID(ccode), include_image)
        .await?
        .ok_or_else(|| {
            AppError::Generic("ccode returned no profile results. enrollment failed.".to_string())
        })?;

    let img = s_res
        .image
        .ok_or_else(|| {
            AppError::Generic("Could not download profile image. enrollment failed".to_string())
        })?
        .bytes
        .ok_or_else(|| {
            AppError::Generic(
                "could not read image, empty or malformed. enrollment failed".to_string(),
            )
        })?;

    if img.is_empty() {
        return Err(AppError::Generic("image has no size. enrollment failed".to_string()));
    }

    let details = s_res.details.ok_or_else(|| {
        AppError::Generic("Could not load profile details. enrollment failed".to_string())
    })?;

    Ok(EnrollData { image: Some(img), details: Some(EnrollDetails::TPass(details)) })
}

//TPASS sends an older structure for enrollment.
#[derive(Serialize, Deserialize, Debug)]
pub struct TPassCandidate {
    pub ccode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_or_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comp_id: Option<String>,
}

//NOTE: candidates is a vec but pretty sure tpass only sends one at a time.
#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollCommand {
    pub command: String,
    pub candidates: Vec<TPassCandidate>,
}

impl From<EnrollCommand> for EnrollData {
    fn from(value: EnrollCommand) -> Self {
        todo!()
    }
}

impl From<&EnrollCommand> for EnrollData {
    fn from(value: &EnrollCommand) -> Self {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollmentResultV1 {
    pub dupe_count: u32,
    pub duplicates: Vec<DupeItem>,
    pub enroll_count: u32,
    pub no_img_count: u32,
    pub rec_fail_count: u32,
    pub search_count: u32,
}

impl Default for EnrollmentResultV1 {
    fn default() -> Self {
        Self {
            dupe_count: 0,
            duplicates: vec![],
            enroll_count: 1,
            no_img_count: 0,
            rec_fail_count: 0,
            search_count: 1,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct DupeItem {
    pub ccode: String,
    pub identities: Vec<Value>,
}

impl Default for DupeItem {
    fn default() -> Self {
        Self {
            ccode: "0".to_string(),
            identities: vec![json!({
                "id": "123abc456def",
                "created_at": "2023-01-01T01:01:00", //these aren't useful.
                "updated_at": "2023-01-01T01:01:00", //dummy vals
                "confidence": 0.90
            })],
        }
    }
}

pub async fn delete_enrollment_v1(
    State(app_state): State<AppState>,
    Json(del_req): Json<DeleteEnrollmentsRequestV1>,
) -> WResult<Json<Value>> {
    let fr_id = del_req
        .fr_ids
        .first()
        .ok_or_else(|| AppError::Generic("No fr_id was found. Did you send one?".to_string()))?;

    let del_res = app_state
        .fr_service
        .delete_enrollment(&fr_id)
        .await
        .inspect_err(|e| error!(target: "enrollment", "{}", e));

    let res = match del_res {
        Ok(v) => {
            info!("1️⃣ deleted enrollment:  fr_id: {}", v.fr_id);
            json!({
                "delete_results": [
                    {
                        "fr_id": &fr_id,
                        "msg": "",
                        "result": "success"
                    }
                ]
            })
        }
        Err(e) => {
            json!({
                "delete_results": [
                    {
                        "fr_id": &fr_id,
                        "msg": e.to_string(),
                        "result": "fail"
                    }
                ]
            })
        }
    };

    Ok(Json(json!(res)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteEnrollmentsRequestV1 {
    pub fr_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_delete: Option<bool>, //includes requesting delete to linked servers like tpass
}

pub async fn add_face_v1(
    State(app_state): State<AppState>,
    Query(params): Query<QParams>,
    multipart: Multipart,
) -> WResult<Json<AddFaceResponseV1>> {
    let fr_id = params.fr_id.ok_or_else(|| {
        AppError::Generic("fr_id query param is empty. what would we be adding?".to_string())
    })?;

    let mut face_req = extractors::extract_add_face_form_data(multipart).await?;
    face_req.fr_id = fr_id;

    let res = app_state.fr_service.add_face(&face_req.fr_id, face_req.image.unwrap()).await?;

    Ok(Json(AddFaceResponseV1 { fr_id: res.fr_id, face_id: res.face_id }))
}

#[derive(Debug, Deserialize)]
pub struct QParams {
    //#[serde(default, deserialize_with = "empty_string_as_none")]
    pub fr_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFaceResponseV1 {
    pub face_id: String,
    pub fr_id: String,
}

pub async fn delete_face_v1(
    State(app_state): State<AppState>,
    Json(req): Json<DeleteFaceBy>,
) -> WResult<Json<DeleteFaceBy>> {
    if req.face_id.trim().is_empty() || req.fr_id.trim().is_empty() {
        return Err(AppError::Generic(
            "must provide fr_id and face_id to delete a secondary face".to_string(),
        ));
    }

    let res = app_state.fr_service.delete_faces(&req.fr_id, vec![req.face_id.clone()]).await?;

    Ok(Json(req))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteFaceBy {
    pub fr_id: String,
    pub face_id: String,
}

pub async fn get_faces(
    State(app_state): State<AppState>,
    Json(req): Json<GetFacesRequest>,
) -> WResult<Json<Vec<EnrolledFaceInfo>>> {
    if req.fr_id.is_empty() {
        return Err(AppError::Generic("fr_id was empty. Did you send one?".to_string()));
    }
    let faces_info = app_state.fr_service.get_faces(req.fr_id.as_str()).await?;
    Ok(Json(faces_info))
}
pub async fn get_faces_v1(
    State(app_state): State<AppState>,
    Json(req): Json<GetFacesRequest>,
) -> WResult<Json<Value>> {
    if req.fr_id.is_empty() {
        return Err(AppError::Generic("fr_id was empty. Did you send one?".to_string()));
    }
    let faces_info = app_state.fr_service.get_faces(req.fr_id.as_str()).await?;
    let face_vals: Vec<Value> = faces_info
        .into_iter()
        .map(|fi| {
            json!({
                "id": fi.face_id,
                "created_at": fi.created_at,
                "quality": fi.quality

            })
        })
        .collect();

    let resp = json!({
      "faces": face_vals,
      "next_page_token": "",
      "total_size": face_vals.len()
    });

    Ok(Json(resp))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetFacesRequest {
    pub fr_id: String,
}
