use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use serde_json::{json, Value};
use tracing::info;

use crate::{extractors, AppState, WResult};
use libfr::backend::MatchConfig;
//use libfr::Face;

pub async fn quality_check(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data.image.unwrap();

    let face = app_state.fr_service.get_closest_face(image, false).await?;
    let quality = face.quality.unwrap_or(0.0);
    let acceptability = face.acceptability.unwrap_or(0.0);

    let pass = quality >= app_state.config.min_quality
        && acceptability >= app_state.config.min_acceptability;

    Ok(Json(json!({
        "high_quality": pass,
        "image": {
            "min_acceptability": app_state.config.min_acceptability,
            "min_quality": app_state.config.min_quality,
            "acceptability": acceptability,
            "quality": quality,
        },
    })))
}
/// Spoof check flag is currently passed through to backend implementation.
pub async fn liveness_check(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data.image.unwrap();

    let face = app_state.fr_service.get_closest_face(image, true).await?;

    let min_acceptability = app_state.config.min_acceptability;
    let min_quality = app_state.config.min_quality;
    let quality = face.quality.unwrap_or(0.0);
    let acceptability = face.acceptability.unwrap_or(0.0);

    let liveness = face.liveness.unwrap_or(libfr::Liveness {
        is_live: false,
        feedback: vec!["LIVENESS_NOT_AVAILABLE".to_string()],
        score: 0.0,
    });

    //We have a demo that depends on this but i think this is a confusing result
    Ok(Json(json!({
        "image": {
            "min_acceptability": min_acceptability,
            "min_quality": min_quality,
            "acceptability": acceptability,
            "quality": quality,
        },
        "face": {
            "bounding_box": face.bbox,
        },
        "liveness": {
            "min_score": 0.5,
            "score": liveness.score,
            "feedback": liveness.feedback,
            "is_live": liveness.is_live,
        },
        "is_valid": is_image_valid(
            acceptability,
            liveness.score,
            liveness.is_live,
            &liveness.feedback,
            min_acceptability,
        ),
    })))
}

pub async fn detect_faces(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    let image = img_data.image.unwrap();

    //NOTE:do we need to use imageOpts?
    let mut faces = app_state.fr_service.detect_faces(image, false).await?;

    for f in &mut faces {
        f.liveness = None
    }

    Ok(Json(json!(faces)))
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
    mconf.top_n = img_data.opts.as_ref().map_or(mconf.top_n, |opts| opts.top_matches as i32);
    let image = img_data.image.unwrap();

    let mut identities = app_state.fr_service.recognize(image, mconf).await?;

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

    Ok(Json(json!(identities)))
}

fn is_image_valid(
    acceptability: f32,
    liveness_score: f32,
    is_live: bool,
    feedback: &[String],
    min_acceptability: f32,
) -> bool {
    if acceptability < min_acceptability {
        return false;
    }

    if liveness_score < 0.5 {
        return false;
    }

    if !is_live {
        return false;
    }

    if !feedback.is_empty() {
        return false;
    }

    true
}
