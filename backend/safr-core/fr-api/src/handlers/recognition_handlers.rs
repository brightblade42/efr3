use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use serde_json::{json, Value};

use crate::{extractors, AppState, WResult};
use libfr::types::{FRIdentity, Liveness, MatchConfig};

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

    let liveness = face.liveness.unwrap_or(Liveness {
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
        f.liveness = None;
        f.template = None;
    }

    Ok(Json(json!(faces)))
}

// Recognize a face and return information about that face and details about the person
/// it is most likely to be.
pub async fn recognize(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let mut mconf = MatchConfig::from(&app_state.config);

    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    if let Some(opts) = &img_data.opts {
        mconf.top_n = opts.top_matches as i32;
        mconf.include_details = opts.include_details;
    }
    let image = img_data.image.unwrap();

    let mut identities = app_state.fr_service.recognize(image, mconf).await?;

    if identities.len() > 1 {
        identities.sort_by(|a, b| {
            let x1 = a.face.bbox.as_ref().map_or(f32::MAX, |bbox| bbox.origin.x);
            let x2 = b.face.bbox.as_ref().map_or(f32::MAX, |bbox| bbox.origin.x);
            x1.partial_cmp(&x2).unwrap_or(std::cmp::Ordering::Equal)
        });
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

//V1 Compat
pub async fn recognize_v1(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let mut mconf = MatchConfig::from(&app_state.config);

    let mut img_data =
        extractors::extract_image_data(multipart, app_state.config.min_match).await?;

    if let Some(opts) = img_data.opts.as_mut() {
        mconf.top_n = opts.top_matches as i32;
        mconf.include_details = true
    }

    let image = img_data.image.unwrap();
    let identities = app_state.fr_service.recognize(image, mconf).await?;

    Ok(Json(json!(to_recognize_v1(identities))))
}

fn to_recognize_v1(fr_idents: Vec<FRIdentity>) -> Value {
    let identities: Vec<Value> = fr_idents
        .into_iter()
        .filter(|x| !x.possible_matches.is_empty())
        .map(|x| {
            let pm = &x.possible_matches[0];

            json!({
                "id": pm.fr_id,
                "created_at": "2023-01-01T01:01:00", //these aren't useful.
                "updated_at": "2023-01-01T01:01:00", //dummy vals
                "confidence": pm.score //TODO: make sure this maps onto what v1 expects score might be a different scale.
            })
        })
        .collect();

    let res = json!({

        "face_count": identities.len(),
        "identities": identities
    });

    res
}
