use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use serde_json::Value;
use tracing::info;

use crate::{errors::AppError::Generic, extractors, AppState, WResult};
use libfr::backend::MatchConfig;

/// Spoof check flag is currently passed through to backend implementation.
pub async fn detect_spoof_image(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data
        .image
        .ok_or_else(|| Generic("An image is required but was not provided".to_string()))?;
    let res = app_state.backend.detect_face(image, true).await?;
    Ok(Json(res))
}

pub async fn detect_image(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data
        .image
        .ok_or_else(|| Generic("An image is required but was not provided".to_string()))?;
    let res = app_state.backend.detect_face(image, false).await?;
    Ok(Json(res))
}

/// Recognize a face and return information about that face and details about the person
/// it is most likely to be.
pub async fn recognize(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let want_details = true;
    let mut mconf = MatchConfig::from(&app_state.config);

    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    mconf.top_n = img_data
        .opts
        .as_ref()
        .map_or(mconf.top_n, |opts| opts.top_matches as i32);

    let image = img_data
        .image
        .ok_or_else(|| Generic("An image is required but was not provided".to_string()))?;

    let mut identities = app_state.backend.recognize(image, mconf).await?;

    if identities.len() > 1 {
        identities.sort_by(|a, b| {
            let x1 = a.face.bbox.as_ref().map_or(f32::MAX, |bbox| bbox.origin.x);
            let x2 = b.face.bbox.as_ref().map_or(f32::MAX, |bbox| bbox.origin.x);
            x1.partial_cmp(&x2).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    if want_details {
        info!("details for all possible matches is wanted");
    }

    let value = serde_json::to_value(identities)
        .map_err(|e| Generic(format!("failed to serialize recognition result: {}", e)))?;
    Ok(Json(value))
}
