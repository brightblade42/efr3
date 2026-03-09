use axum::{
    extract::{multipart::Multipart, Query, State},
    Json,
};
use libfr::backend::MatchConfig;
use libfr::repo::EnrollmentMetadataRecord;
use libfr::{
    AddFaceResult, EnrollmentDeleteResult, EnrollmentRosterItem, GetFaceInfoResult, IDPair,
    ResetEnrollmentsResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::{errors::AppError::Generic, extractors, AppState, WResult};

pub async fn search_enrollment(
    State(app_state): State<AppState>,
    Json(search_by): Json<SearchEnrollmentBy>,
) -> WResult<Json<Vec<EnrollmentRosterItem>>> {
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
pub async fn get_enrollment_roster(
    State(app_state): State<AppState>,
) -> WResult<Json<Vec<EnrollmentRosterItem>>> {
    let res = app_state.fr_service.get_enrollment_roster().await?;
    Ok(Json(res))
}

pub async fn delete_enrollment(
    State(app_state): State<AppState>,
    Json(payload): Json<DeleteEnrollmentBy>,
) -> WResult<Json<EnrollmentDeleteResult>> {
    //this is a little weird
    let fr_id = resolve_fr_id_for_delete(&app_state, payload).await?;
    let res = app_state.fr_service.delete_enrollment(&fr_id).await?;
    info!("{:?}", res);
    Ok(Json(res))
}

/// Deletes all enrollments and resets everything.
pub async fn reset_enrollments(
    State(app_state): State<AppState>,
) -> WResult<Json<ResetEnrollmentsResult>> {
    let res = app_state.fr_service.reset_enrollments().await?;
    Ok(Json(res))
}

pub async fn add_face(
    State(app_state): State<AppState>,
    Query(query): Query<AddFaceQuery>,
    multipart: Multipart,
) -> WResult<Json<AddFaceResult>> {
    let fr_id = query.fr_id.trim();
    if fr_id.is_empty() {
        return Err(Generic("fr_id query param is required for add-face".to_string()));
    }

    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data
        .image
        .ok_or_else(|| Generic("No image was provided for add-face".to_string()))?;

    let res = app_state.fr_service.add_face(fr_id, image).await?;
    Ok(Json(res))
}

pub async fn delete_face(
    State(app_state): State<AppState>,
    Json(req): Json<DeleteFaceRequest>,
) -> WResult<Json<DeleteFaceApiResponse>> {
    if req.fr_id.trim().is_empty() || req.face_id.trim().is_empty() {
        return Err(Generic("fr_id and face_id are required to delete a face".to_string()));
    }

    let res = app_state.fr_service.delete_face(&req.fr_id, &req.face_id).await?;
    Ok(Json(DeleteFaceApiResponse {
        rows_affected: res.rows_affected,
        fr_id: req.fr_id,
        face_id: req.face_id,
    }))
}

pub async fn get_face_info(
    State(app_state): State<AppState>,
    Json(req): Json<GetFaceInfoRequest>,
) -> WResult<Json<GetFaceInfoResult>> {
    if req.fr_id.trim().is_empty() {
        return Err(Generic("fr_id is required to get face info".to_string()));
    }

    let res = app_state.fr_service.get_face_info(&req.fr_id).await?;
    Ok(Json(res))
}

pub async fn get_enrollment_errlog(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let logs = app_state
        .fr_repo
        .get_enrollment_logs(100)
        .await
        .map_err(|e| Generic(format!("failed to load enrollment logs: {}", e)))?;

    let value = serde_json::to_value(logs)
        .map_err(|e| Generic(format!("failed to serialize enrollment logs: {}", e)))?;

    Ok(Json(value))
}

/// Gets metadata about the enrollment database.
pub async fn get_enrollment_metadata(
    State(app_state): State<AppState>,
) -> WResult<Json<EnrollmentMetadataRecord>> {
    let res = app_state.fr_service.get_enrollment_metadata().await?;
    Ok(Json(res))
}

/// A collection is another term for "gallery" or "roster".
pub async fn create_collection(State(_app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(json!({ "msg": "not yet implemented"})))
}

async fn resolve_fr_id_for_delete(
    app_state: &AppState,
    del_by: DeleteEnrollmentBy,
) -> WResult<String> {
    match del_by {
        DeleteEnrollmentBy::FrId(id) => {
            let trimmed = id.trim();
            if trimmed.is_empty() {
                return Err(Generic("fr_id was provided but empty".to_string()));
            }
            Ok(trimmed.to_string())
        }
        DeleteEnrollmentBy::CCode(ccode) => {
            let ext_id = ccode.to_string();

            let profile = app_state
                .fr_repo
                .get_profile_by_ext_id(&ext_id)
                .await
                .map_err(|e| {
                    Generic(format!(
                        "failed to resolve enrollment by external id {}: {}",
                        ext_id, e
                    ))
                })?
                .ok_or_else(|| {
                    Generic(format!("no enrollment found for external id {}", ext_id))
                })?;

            //this wouldn't be possible
            profile
                .fr_id
                .ok_or_else(|| Generic(format!("profile for external id {} has no fr_id", ext_id)))
        }
        DeleteEnrollmentBy::Name(first, last) => {
            let profile = app_state
                .fr_repo
                .find_profile_by_name(&first, &last, None)
                .await
                .map_err(|e| {
                    Generic(format!(
                        "failed to resolve enrollment by name '{}, {}': {}",
                        last, first, e
                    ))
                })?
                .ok_or_else(|| {
                    Generic(format!("no enrollment found for name '{}, {}'", last, first))
                })?;

            profile.fr_id.ok_or_else(|| {
                Generic(format!("profile for name '{}, {}' has no fr_id", last, first))
            })
        }
        DeleteEnrollmentBy::FullName(full) => {
            let profile = app_state
                .fr_repo
                .find_profile_by_name(&full.first, &full.last, full.middle.as_deref())
                .await
                .map_err(|e| {
                    Generic(format!(
                        "failed to resolve enrollment by full name '{}, {}': {}",
                        full.last, full.first, e
                    ))
                })?
                .ok_or_else(|| {
                    Generic(format!(
                        "no enrollment found for full name '{}, {}'",
                        full.last, full.first
                    ))
                })?;

            profile.fr_id.ok_or_else(|| {
                Generic(format!(
                    "profile for full name '{}, {}' has no fr_id",
                    full.last, full.first
                ))
            })
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
    CCode(u64),
    Name(String, String),
    FullName(FullName),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum SearchEnrollmentBy {
    #[serde(rename = "last_name")]
    LastName(String),
}

#[derive(Deserialize, Debug)]
pub(crate) struct AddFaceQuery {
    pub fr_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DeleteFaceRequest {
    pub fr_id: String,
    pub face_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DeleteFaceApiResponse {
    pub rows_affected: i32,
    pub fr_id: String,
    pub face_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GetFaceInfoRequest {
    pub fr_id: String,
}
