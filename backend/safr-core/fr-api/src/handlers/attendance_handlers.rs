use crate::json_str;
use crate::{errors::AppError::Generic, extractors, AppState, WResult};
use axum::{
    extract::{multipart::Multipart, State},
    Json,
};
use libfr::{backend::MatchConfig, FRIdentity};
use libtpass::types::AttendanceKind;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

/// mark_attendance will recognize a face in an image and notify the remote (tpass) that someone
/// has entered or exited a building or room.
pub async fn mark_attendance(
    State(app_state): State<AppState>,
    multipart: Multipart,
) -> WResult<Json<Value>> {
    let mut mconf = MatchConfig::from(&app_state.config);
    let img_data = extractors::extract_image_data(multipart, app_state.config.min_match).await?;

    if let Some(opts) = &img_data.opts {
        mconf.top_n = opts.top_matches as i32;
        mconf.include_details = opts.include_details;
    }
    // Image unwrap is safe due to extract_image_data guard
    let res = app_state.fr_service.recognize(img_data.image.unwrap(), mconf).await?;

    if res.is_empty() {
        debug!("mark_attendance: recognition produced nothing so we bail empty array returned.");
        return Ok(Json(json!({})));
    }

    //not likely to occur
    if res.len() > 1 {
        warn!(
            "mark_attendance identified {} faces. This is not supported in this version",
            res.len()
        );
    }

    let fr_ident = res.into_iter().next().unwrap();
    let details = validate_details(&fr_ident)?;

    // 2. Early return validation with `let else`
    let Some(idnum) = details["idnumber"].as_str().filter(|s| !s.is_empty()) else {
        return Err(Generic(
            "A client idnumber is required to record attendance. check with tpass".to_string(),
        ));
    };

    let ccode = parse_ccode(&details).ok_or_else(|| {
        Generic("A valid client ccode is required to record attendance".to_string())
    })?;

    let idpair = (idnum.to_string(), ccode);
    // 3. Immutably unpack options using map_or
    let (att_name, att_kind, location) =
        img_data.opts.map_or((String::new(), None, String::new()), |opt| {
            let kind = match opt.on_match.as_str() {
                "check_in" => Some(AttendanceKind::In),
                "check_out" => Some(AttendanceKind::Out),
                _ => None,
            };
            (opt.on_match, kind, opt.rec_location)
        });

    //tell TPASS we're checking in or out.
    let status = match att_kind {
        Some(kind) => app_state
            .tpass_client
            .mark_attendance(idpair, kind)
            .await
            .map_err(|e| Generic(format!("couldn't mark attendance: {}", e)))?,
        None => None,
    };

    let extra = serde_json::to_value(&status).ok();

    let pm = fr_ident.possible_matches.first().unwrap();

    app_state.fr_service.log_cam_fr_match(&pm, extra.as_ref(), &location).await?;
    let client_name = format!("{} {}", json_str!(details, "fName"), json_str!(details, "lName"));
    info!(target: "attendance", "{} | {} | {} | {}", att_name, pm.fr_id, client_name,  location );

    Ok(Json(json!({
        "identity": fr_ident,
        "status": status
    })))
}

//make sure we have the personal information required for top match
fn validate_details(fr_ident: &FRIdentity) -> WResult<Value> {
    fr_ident
        .possible_matches
        .first()
        .and_then(|x| x.details.clone())
        .ok_or_else(|| {
            error!(target: "attendance", "recognized face does not have any saved details.");
            Generic("recognized face has no saved details. Can't mark for attendance".to_string())
        })
}

fn parse_ccode(details: &Value) -> Option<u64> {
    details.get("ccode").and_then(Value::as_u64)
}
