use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{errors::AppError, errors::StandardError, AppState, WResult};
use libtpass::types::{FRAlert, SearchRequest};

#[derive(Debug, Deserialize)]
pub struct SearchTpassRequest {
    #[serde(flatten)]
    pub search: SearchRequest,
    #[serde(default)]
    pub verbose: bool,
}

//get a bunch of people info from TPass, this is a passthrough. I wonder if our client tools
//should just contact tpass directly?
pub async fn search_tpass(
    State(app_state): State<AppState>,
    Json(search_req): Json<SearchTpassRequest>,
) -> WResult<Json<Value>> {
    let verbose = search_req.verbose;
    let res = app_state
        .tpass_client
        .search_tpass_verbose(search_req.search)
        .await;
    match res {
        Ok(tpr) => {
            if tpr.meta.failed > 0 {
                warn!(
                    "search_tpass partial failure: {}/{} failed",
                    tpr.meta.failed, tpr.meta.attempted
                );
            }
            let mut total = 0;
            for oa in tpr.items.iter() {
                total += oa.as_array().map_or(0, |arr| arr.len());
            }
            info!("Search Size: {}", total);

            if verbose {
                let payload = serde_json::to_value(tpr).map_err(|e| {
                    tpass_passthrough_error("couldn't serialize verbose search response", e)
                })?;
                Ok(Json(payload))
            } else {
                let payload = serde_json::to_value(tpr.items).map_err(|e| {
                    tpass_passthrough_error("couldn't serialize search response", e)
                })?;
                Ok(Json(payload))
            }
        }
        Err(e) => {
            warn!("search_tpass request failed: {}", e);

            if verbose {
                Err(tpass_passthrough_error("search_tpass request failed", e))
            } else {
                Ok(Json(json!([{}])))
            }
        }
    }
}

pub async fn send_fr_alert(
    State(app_state): State<AppState>,
    Json(req): Json<FRAlert>,
) -> WResult<Json<Value>> {
    let res = app_state
        .tpass_client
        .send_fr_alert(req)
        .await
        .map_err(|e| tpass_passthrough_error("couldn't send fr alert", e))?;

    info!("fr alert returned a success");
    Ok(Json(res))
}

///TPass has a number of companies that we need to be aware of .
pub async fn get_tpass_companies(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    info!("getting tpass companies");
    let res = app_state
        .tpass_client
        .get_companies()
        .await
        .map_err(|e| tpass_passthrough_error("couldn't get companies list", e))?;
    Ok(Json(res))
}

///TPass has a number of client types that we need to be aware of
pub async fn get_tpass_client_types(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state
        .tpass_client
        .client_types()
        .await
        .map_err(|e| tpass_passthrough_error("couldn't get tpass client types list", e))?;
    Ok(Json(res))
}

///TPass has a number of status types that we need to be aware of.
pub async fn get_tpass_status_types(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    let res = app_state
        .tpass_client
        .status_types()
        .await
        .map_err(|e| tpass_passthrough_error("couldn't get tpass status types list", e))?;
    Ok(Json(res))
}

fn tpass_passthrough_error(message: &str, source: impl std::fmt::Display) -> AppError {
    AppError::Standard(StandardError {
        code: 9000,
        message: message.to_string(),
        details: Some(json!({ "source": source.to_string() })),
    })
}
