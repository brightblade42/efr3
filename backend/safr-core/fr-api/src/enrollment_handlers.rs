use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use libfr::backend::{FRBackend, MatchConfig};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::{extractors, AppState, WResult};

pub async fn search_enrollment(
    State(app_state): State<AppState>,
    Json(search_by): Json<SearchEnrollmentBy>,
) -> WResult<Json<Vec<Value>>> {
    let SearchEnrollmentBy::LastName(term) = search_by;

    let res = app_state
        .fr_engine
        .get_enrollments_by_last_name(&term)
        .await?;
    Ok(Json(res))
}

// FR enrollment flow: image + details are transformed and sent to backend service.
pub async fn create_enrollment(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let enroll_data = extractors::extract_enroll_data(multipart).await?;
    let mconf = MatchConfig::from(&app_state.config);

    let res = app_state
        .fr_engine
        .create_enrollment(enroll_data, mconf)
        .await?;
    Ok(Json(res))
}

/// Returns a list of every enrollment in the system. We will want to add paging.
pub async fn get_enrollment_roster(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state.fr_engine.get_enrollment_roster().await?;
    Ok(Json(res))
}

pub async fn delete_enrollment(
    app_state: State<AppState>,
    Json(payload): Json<DeleteEnrollmentBy>,
) -> WResult<Json<Value>> {
    let image_id = extract_image_id(payload);
    let res = app_state.fr_engine.delete_enrollment(&image_id).await?;
    info!("{:?}", res);
    Ok(Json(res))
}

/// Deletes all enrollments and resets everything.
pub async fn reset_enrollments(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state.fr_engine.reset_enrollments().await?;
    Ok(Json(res))
}

pub async fn get_enrollment_errlog(State(_app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(json!({ "msg": "coming soon!"})))
}

/// Gets metadata about the enrollment database.
pub async fn get_enrollment_metadata(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state.fr_engine.get_enrollment_metadata().await?;
    Ok(Json(res))
}

/// A collection is another term for "gallery" or "roster".
pub async fn create_collection(State(_app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(json!({ "msg": "not yet implemented"})))
}

// There are a few different kinds of data we can send that we will convert to a face id.
// This fn matches the type sent and gets the proper id based on that.
// TODO: Not ready for this just yet. We will need to complete the conversions.
fn extract_image_id(del_by: DeleteEnrollmentBy) -> String {
    match del_by {
        DeleteEnrollmentBy::FrId(id) => id,
        DeleteEnrollmentBy::CCode(id) => {
            // get fr id from db on matching ccode
            id
        }
        DeleteEnrollmentBy::FullName(_fname) => "12345".to_string(),
        DeleteEnrollmentBy::Name(_f, _l) => "54321".to_string(),
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
    CCode(String),
    Name(String, String),
    FullName(FullName),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum SearchEnrollmentBy {
    #[serde(rename = "last_name")]
    LastName(String),
}
