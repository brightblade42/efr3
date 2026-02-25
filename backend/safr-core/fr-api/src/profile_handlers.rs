use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use libfr::{backend::MatchConfig, EnrollData, EnrollDetails, EnrollmentCreateResult};
use libtpass::types::EditProfileRequest;
use serde_json::Value;

use crate::{errors::AppError::Generic, extractors, AppState, WResult};

/// Edit an existing remote profile. This useful for things like update a person to the
/// FR watch list.
pub async fn edit_profile(
    State(app_state): State<AppState>,
    Json(edit_profile_req): Json<EditProfileRequest>,
) -> WResult<Json<Value>> {
    let res = app_state
        .tpass_client
        .edit_profile(edit_profile_req)
        .await?;

    Ok(Json(res))
}

/// Create profile with remote and then enroll. This is how a gui client can
/// enroll a new person from a face that has been detected.
pub async fn create_profile(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<EnrollmentCreateResult>> {
    let profile_data = extractors::extract_new_profile_req(multipart).await?;

    let np_resp = app_state
        .tpass_client
        .create_profile(&profile_data.profile)
        .await?;
    let mut ep_req = EditProfileRequest::from(&np_resp);
    ep_req.state = Some("".to_string()); //satisfy the fickle TPASS gods.

    let _ep_resp = app_state.tpass_client.edit_profile(ep_req).await?;
    let tp_res = app_state
        .tpass_client
        .get_clients_by_ccode(vec![np_resp.ccode])
        .await?;

    let tp_val = serde_json::to_value(tp_res.first())
        .map_err(|e| Generic(format!("failed to serialize profile details: {}", e)))?;
    let enroll_details = EnrollDetails::TPass(tp_val);

    let enroll_data = EnrollData {
        image: Some(profile_data.image.clone()),
        details: Some(enroll_details),
    };

    let mconf = MatchConfig::from(&app_state.config);
    let res = app_state
        .fr_service
        .create_enrollment(enroll_data, mconf)
        .await?;

    Ok(Json(res))
}
