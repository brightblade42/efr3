use axum::{
    Json,
    extract::{State, multipart::Multipart},
};
use libfr::tpass::types::EditProfileRequest;
use libfr::types::{EnrollData, EnrollDetails, IDPair, MatchConfig};
use serde_json::Value;

use crate::{
    AppState, WResult,
    errors::AppError::{self},
    extractors,
};

/// Edit an existing remote profile. This useful for things like update a person to the
/// FR watch list.
pub async fn edit_profile(
    State(app_state): State<AppState>,
    Json(edit_profile_req): Json<EditProfileRequest>,
) -> WResult<Json<Value>> {
    //let res = app_state.tpass_client.edit_profile(edit_profile_req).await?;
    todo!()
    //Ok(Json(res))
}

/// Create profile with remote and then enroll. This is how a gui client can
/// enroll a new person from a face that has been detected.
pub async fn create_profile(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<IDPair>> {
    todo!();
    //let profile_data = extractors::extract_new_profile_req(multipart).await?;

    //TODO: Remember why we have to create and edit a remote profile.
    // let np_resp = app_state.fr_service.assets.create_profile(&profile_data.profile).await?;
    // let mut ep_req = EditProfileRequest::from(&np_resp);
    // ep_req.state = Some("".to_string()); //satisfy the fickle TPASS gods.

    // let _ep_resp = app_state.tpass_client.edit_profile(ep_req).await?;
    // let tp_res = app_state.tpass_client.get_clients_by_ccode(vec![np_resp.ccode]).await?;

    // let prof = tp_res.into_iter().next().ok_or_else(|| {
    //     //TODO: create an AppError::RemoteProfileNotFound
    //     AppError::Generic(format!("could not load profile for client with ccode {}", np_resp.ccode))
    // })?;

    // let enroll_data = EnrollData {
    //     image: Some(profile_data.image.clone()),
    //     details: Some(EnrollDetails::TPass(prof)),
    // };

    // let mconf = MatchConfig::from(&app_state.config);
    // let res = app_state.fr_service.create_enrollment(&enroll_data, mconf).await?;

    //Ok(Json(res))
}
