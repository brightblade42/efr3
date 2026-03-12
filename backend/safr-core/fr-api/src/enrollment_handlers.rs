use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use libfr::repo::EnrollmentMetadataRecord;
use libfr::{backend::MatchConfig, errors::FRError};
use libfr::{EnrolledFaceInfo, EnrollmentDeleteResult, IDPair};
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
pub async fn get_enrollment_roster(State(app_state): State<AppState>) -> WResult<Json<Vec<Value>>> {
    let x = app_state.fr_service.get_enrollment_roster().await?;
    Ok(Json(x))
}

pub async fn delete_enrollment(
    State(app_state): State<AppState>,
    Json(payload): Json<DeleteEnrollmentBy>,
) -> WResult<Json<EnrollmentDeleteResult>> {
    //this is a little weird
    //let fr_id = resolve_fr_id_for_delete(&app_state, payload).await?;
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
pub(crate) struct FullName {
    first: String,
    middle: Option<String>,
    last: String,
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
