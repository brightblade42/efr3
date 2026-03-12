use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{errors::AppError, errors::StandardError, AppState, WResult};
use libtpass::{
    errors::TPassError,
    types::{FRAlert, SearchRequest},
};

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
    let res = app_state.tpass_client.search_tpass(search_req.search).await;
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

            Ok(Json(
                if verbose { serde_json::to_value(&tpr) } else { serde_json::to_value(&tpr.items) }
                    .map_err(|e| {
                        TPassError::Generic(format!("couldn't serialize search response: {}", e))
                    })?,
            ))
        }
        Err(e) => {
            warn!("search_tpass request failed: {}", e);
            if verbose {
                Err(AppError::Generic(format!("search_tpass request failed: {}", e)))
            } else {
                Ok(Json(json!([])))
            }
        }
    }
}

pub async fn send_fr_alert(
    State(app_state): State<AppState>,
    Json(req): Json<FRAlert>,
) -> WResult<Json<Value>> {
    Ok(Json(
        app_state
            .tpass_client
            .send_fr_alert(req)
            .await
            .inspect(|_| info!("fr alert returned a success"))?,
    ))
}

///TPass has a number of companies that we need to be aware of .
pub async fn get_tpass_companies(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(app_state.tpass_client.get_companies().await?))
}

///TPass has a number of client types that we need to be aware of
pub async fn get_tpass_client_types(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(app_state.tpass_client.client_types().await?))
}

///TPass has a number of status types that we need to be aware of.
pub async fn get_tpass_status_types(State(app_state): State<AppState>) -> WResult<Json<Value>> {
    Ok(Json(app_state.tpass_client.status_types().await?))
}
