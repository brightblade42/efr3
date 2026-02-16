use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use libfr::{
    backend::{FRBackend, MatchConfig},
    FRIdentity,
};
use libtpass::types::{AttendanceKind, AttendanceStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

use crate::{errors::AppError, errors::AppError::Generic, extractors, AppState, WResult};

// NOTE: This is called recognize_faces_b64 in V1.
// The image sent to this endpoint should be a single face, but we can't know that for sure.
/// mark_attendance will recognize a face in an image and notify the remote (tpass) that someone
/// has entered or exited a building or room.
pub async fn mark_attendance(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let mut mconf = MatchConfig::from(&app_state.config);
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;
    mconf.top_n = img_data
        .opts
        .as_ref()
        .map_or(mconf.top_n, |opts| opts.top_matches as i32);

    let image = img_data
        .image
        .ok_or_else(|| Generic("An image is required but was not provided".to_string()))?;
    let res = app_state.fr_engine.recognize(image, mconf).await?;

    // TODO: Consider how to better handle recognition results. We only want to deal with a
    // single face. If we have more than one face then we have false positives or an image with
    // multiple faces which is not what we want. (at least in this version)
    let fr_ident = match res.len() {
        0 => {
            debug!(
                "mark_attendance: recognition produced nothing so we bail empty array returned."
            );
            return Ok(Json(json!({})));
        }
        1 => res
            .into_iter()
            .next()
            .ok_or_else(|| Generic("recognition result was unexpectedly empty".to_string()))?,
        _ => {
            warn!(
                "mark_attendance identified {} faces. This is not supported in this version",
                res.len()
            );
            res.into_iter()
                .next()
                .ok_or_else(|| Generic("recognition result was unexpectedly empty".to_string()))?
        }
    };

    let details = validate_details(&fr_ident)?;

    // Attendance (checkin or out) can only be done if there is an "idnumber" present.
    let idpair = if let Some(idnum) = details["idnumber"].as_str() {
        if idnum.is_empty() {
            return Err(Generic(
                "An client idnumber is required to record attendance. check with tpass".to_string(),
            ));
        }
        let ccode = details["ccode"].as_u64().unwrap_or(0);
        (idnum.to_string(), ccode)
    } else {
        return Err(Generic(
            "An client idnumber is required to record attendance. check with tpass".to_string(),
        ));
    };

    let mut v_ident = VerifiedIdentity {
        identity: fr_ident,
        status: None,
    };

    debug!("THE OPTS: {:?}", img_data.opts);
    let mut att_kind = None;
    let mut location = "".to_string();
    match img_data.opts {
        Some(opt) => {
            att_kind = match opt.on_match.as_str() {
                "check_in" => Some(AttendanceKind::In),
                "check_out" => Some(AttendanceKind::Out),
                _ => None,
            };

            location = opt.rec_location.clone();

            Some(())
        }
        None => None,
    };

    info!("att kind: {:?}", &att_kind);
    v_ident.status = match att_kind {
        Some(kind @ AttendanceKind::In) | Some(kind @ AttendanceKind::Out) => app_state
            .tpass_client
            .mark_attendance(idpair, kind)
            .await
            .map_err(|e| AppError::Generic(format!("couldn't mark attendance: {}", e)))?,
        _ => None,
    };

    let extra = serde_json::to_value(&v_ident.status).ok();
    app_state
        .fr_engine
        .log_identity(&v_ident.identity, extra.as_ref(), &location)
        .await?;

    let value = serde_json::to_value(v_ident)
        .map_err(|e| Generic(format!("failed to serialize attendance result: {}", e)))?;
    Ok(Json(value))
}

// Make sure an identity has details we can use.
fn validate_details(fr_ident: &FRIdentity) -> WResult<Value> {
    let details = fr_ident
        .possible_matches
        .first()
        .and_then(|x| x.details.clone());

    let details = match details {
        Some(d) => d,
        None => {
            error!("mark_attendance: recognized face does not have any saved details.");
            return Err(Generic(
                "recognized face has no saved details. Can't mark for attendance".to_string(),
            ));
        }
    };

    Ok(details)
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifiedIdentity {
    identity: FRIdentity,
    status: Option<AttendanceStatus>,
}
