use axum::{
    extract::{multipart::Multipart, Query, State},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::Remote,
    EnrollData, EnrollDetails, FRIdentity, IDKind, Image, SearchBy,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    errors::{AppError, StandardError},
    extractors,
    types::{
        AddFaceResponse, AddFaceResponseV1, DeleteEnrollmentsRequestV1, DupeItem, EnrollCommand,
        EnrollmentResultV1, GetFacesRequest,
    },
    AppState, WResult,
};

pub async fn add_face_v1(
    State(app_state): State<AppState>,
    Query(params): Query<QParams>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let fr_id = match params.fr_id {
        None => {
            return Err(AppError::Generic(
                "fr_id query param is empty. what would we be adding?".to_string(),
            ));
        }
        Some(id) => id,
    };

    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;

    let img = match img_data.image {
        None => {
            return Err(AppError::Generic(
                "No image was provided. what would we be adding?".to_string(),
            ));
        }
        Some(img) => img,
    };

    let res = app_state.fr_engine.add_face(&fr_id, img).await?;
    let add_face = serde_json::from_value::<AddFaceResponse>(res)
        .map_err(|e| AppError::Generic(format!("failed to parse add-face response: {}", e)))?;
    let mut res_v1 = AddFaceResponseV1::try_from(add_face).map_err(|e| {
        AppError::Generic(format!(
            "failed to convert add-face response to v1 payload: {}",
            e
        ))
    })?;

    res_v1.fr_id = fr_id;
    let value = serde_json::to_value(res_v1)
        .map_err(|e| AppError::Generic(format!("failed to serialize add-face response: {}", e)))?;
    Ok(Json(value))
}

pub async fn delete_face_v1(
    State(app_state): State<AppState>,
    Json(del_req): Json<DeleteFaceBy>,
) -> WResult<Json<Value>> {
    if del_req.fr_id.is_empty() || del_req.face_id.is_empty() {
        return Err(AppError::Generic(
            "must provide fr_id and face_id to delete a secondary face".to_string(),
        ));
    }

    let _res = app_state
        .fr_engine
        .delete_face(&del_req.fr_id, &del_req.face_id)
        .await?;

    let value = serde_json::to_value(del_req).map_err(|e| {
        AppError::Generic(format!("failed to serialize delete-face payload: {}", e))
    })?;
    Ok(Json(value))
}

pub async fn get_faces_v1(
    State(app_state): State<AppState>,
    Json(req): Json<GetFacesRequest>,
) -> WResult<Json<Value>> {
    if req.fr_id.is_empty() {
        return Err(AppError::Generic(
            "fr_id was empty. Did you send one?".to_string(),
        ));
    }

    let res = app_state.fr_engine.get_face_info(&req.fr_id).await?;
    let value = serde_json::to_value(res)
        .map_err(|e| AppError::Generic(format!("failed to serialize get-faces response: {}", e)))?;
    Ok(Json(value))
}

pub async fn delete_enrollment_v1(
    app_state: State<AppState>,
    Json(del_req): Json<DeleteEnrollmentsRequestV1>,
) -> WResult<Json<Value>> {
    if del_req.fr_ids.is_empty() {
        return Err(AppError::Generic(
            "No fr_id was found. Did you send one?".to_string(),
        ));
    }

    let fr_id =
        del_req.fr_ids.into_iter().next().ok_or_else(|| {
            AppError::Generic("No fr_id was found. Did you send one?".to_string())
        })?;

    let res = app_state.fr_engine.delete_enrollment(&fr_id).await;
    let v1_res = match res {
        Ok(_v) => {
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
                        "msg": e.message,
                        "result": "fail"
                    }
                ]
            })
        }
    };

    info!("{:?}", &v1_res);
    Ok(Json(v1_res))
}

pub async fn recognize_v1(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let mut mconf = MatchConfig::from(&app_state.config);
    let img_data = extractors::extract_image_data_v1(multipart, app_state.config.min_match).await?;

    mconf.top_n = 1;

    let image = img_data.image.ok_or_else(|| {
        AppError::Generic("An image is required but was not provided".to_string())
    })?;

    let res = app_state.fr_engine.recognize(image, mconf).await?;
    let v1_res = to_recognize_v1(res);
    Ok(Json(v1_res))
}

pub async fn create_enrollment_v1(
    State(app_state): State<AppState>,
    Json(en_cmd): Json<EnrollCommand>,
) -> WResult<Json<Value>> {
    if en_cmd.candidates.is_empty() {
        return Err(AppError::Standard(StandardError {
            code: 5000,
            message: "A ccode is required for enrollment".to_string(),
            details: None,
        }));
    }

    let ccode: u64 = en_cmd
        .candidates
        .first()
        .ok_or_else(|| {
            AppError::Standard(StandardError {
                code: 5000,
                message: "A ccode is required for enrollment".to_string(),
                details: None,
            })
        })?
        .ccode
        .clone()
        .parse()
        .map_err(|_| AppError::Generic("provided ccode is not a number".to_string()))?;

    let s_res = app_state
        .tpass_client
        .search_one(SearchBy::ExtID(IDKind::Num(ccode)), true)
        .await?;

    let s_res = match s_res {
        None => {
            return Err(AppError::Generic(
                "ccode returned no results for enrollment".to_string(),
            ));
        }
        Some(sr) => sr,
    };

    let img = match s_res.image {
        None => {
            return Err(AppError::Generic(
                "An image is required for enrollment.".to_string(),
            ));
        }
        Some(img) => match img {
            Image::Binary(bin) => match bin.len() {
                0 => {
                    return Err(AppError::Generic(
                        "binary image has no size. can't enroll.".to_string(),
                    ));
                }
                _ => Some(general_purpose::STANDARD.encode(bin)),
            },
            Image::Base64(b64) => Some(b64),
        },
    };

    let dets = s_res.details.ok_or_else(|| {
        AppError::Generic("TPass search result does not include details".to_string())
    })?;
    let enroll_det = EnrollDetails::TPass(dets);

    info!("====== ENROLL V1 DETAILS =====");
    info!("{:?}", enroll_det);
    let enroll_data = EnrollData {
        image: img,
        details: Some(enroll_det),
    };

    let mconf = MatchConfig::from(&app_state.config);
    match app_state
        .fr_engine
        .create_enrollment(enroll_data, mconf)
        .await
    {
        Ok(_) => {
            let value = serde_json::to_value(EnrollmentResultV1::default()).map_err(|e| {
                AppError::Generic(format!("failed to serialize enrollment response: {}", e))
            })?;
            Ok(Json(value))
        }

        Err(e) => {
            if e.code == 1020 {
                let dupe_item = DupeItem {
                    ccode,
                    ..DupeItem::default()
                };
                let en_res = EnrollmentResultV1 {
                    dupe_count: 1,
                    enroll_count: 0,
                    duplicates: vec![dupe_item],
                    ..EnrollmentResultV1::default()
                };
                let value = serde_json::to_value(en_res).map_err(|ser_err| {
                    AppError::Generic(format!(
                        "failed to serialize duplicate enrollment response: {}",
                        ser_err
                    ))
                })?;
                Ok(Json(value))
            } else {
                error!("not capturing the fail case for v1 enrollment");
                Err(AppError::Generic(
                    "couldn't enroll tpass client".to_string(),
                ))
            }
        }
    }
}

fn to_recognize_v1(fr_idents: Vec<FRIdentity>) -> Value {
    let identities: Vec<Value> = fr_idents
        .into_iter()
        .filter(|x| !x.possible_matches.is_empty())
        .map(|x| {
            let pm = &x.possible_matches[0];

            json!({
                "id": pm.fr_id,
                "created_at": "2023-01-01T01:01:00",
                "updated_at": "2023-01-01T01:01:00",
                "confidence": pm.confidence
            })
        })
        .collect();

    json!({
        "face_count": identities.len(),
        "identities": identities
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DeleteFaceBy {
    fr_id: String,
    face_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QParams {
    fr_id: Option<String>,
}
