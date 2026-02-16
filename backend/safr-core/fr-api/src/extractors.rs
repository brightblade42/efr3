use axum::extract::multipart::Multipart;
use base64::{engine::general_purpose, Engine as _};
use libfr::EnrollData;
use libtpass::types::NewProfileRequest;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use tracing::{debug, info};

use crate::errors::AppError;
use crate::errors::AppError::Generic;

type WResult<T> = Result<T, AppError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageOpts {
    pub top_matches: u8, //how many potential face matches to include (decreasing conf)
    pub include_detected_faces: bool,
    pub on_match: String,     //action to take on a match
    pub min_match: f32,       //threshold of confidence
    pub rec_location: String, //camera stream name
}

impl Default for ImageOpts {
    fn default() -> Self {
        Self {
            top_matches: 1,
            include_detected_faces: false,
            on_match: "id_only".to_string(),
            min_match: 0.90, //get this from env
            rec_location: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageData {
    pub image: Option<String>,
    pub opts: Option<ImageOpts>,
}

/// Extract request parameters and return ImageData struct.
/// NOTE: v1 is needed because TPass does not set content_type.
pub async fn extract_image_data_v1(mut multipart: Multipart, min_match: f32) -> WResult<ImageData> {
    let mut image_data = ImageData {
        image: None,
        opts: None,
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;

                image_data.image = match bytes.len() {
                    0 => None,
                    _ => Some(general_purpose::URL_SAFE.encode(bytes)),
                };
            }

            "opts" => {
                let jval = field.text().await.map_err(|x| Generic(x.to_string()))?;
                debug!("opts: {:?}", &jval);
                let p_res = serde_json::from_str(jval.as_str());
                image_data.opts = match p_res {
                    Ok(opts) => Some(opts),
                    Err(_) => Some(ImageOpts {
                        min_match,
                        ..ImageOpts::default()
                    }),
                }
            }
            _ => {}
        }
    }

    if image_data.image.is_none() {
        return Err(Generic(
            "An image is required but was not provided".to_string(),
        ));
    }

    Ok(image_data)
}

/// Extract image and opts from multipart data.
pub async fn extract_image_data(mut multipart: Multipart, min_match: f32) -> WResult<ImageData> {
    let mut image_data = ImageData {
        image: None,
        opts: None,
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                image_data.image = match field.content_type() {
                    Some(x) if x.starts_with("image") => {
                        let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                        match bytes.len() {
                            0 => None,
                            _ => Some(general_purpose::STANDARD.encode(bytes)),
                        }
                    }
                    _ => {
                        let image_txt = field.text().await.map_err(|x| Generic(x.to_string()))?;
                        match image_txt.len() {
                            0 => None,
                            _ => Some(image_txt),
                        }
                    }
                }
            }

            "opts" => {
                let jval = field.text().await.map_err(|x| Generic(x.to_string()))?;
                info!("opts: {:?}", &jval);
                let p_res = serde_json::from_str(jval.as_str());
                image_data.opts = match p_res {
                    Ok(opts) => Some(opts),
                    Err(_) => Some(ImageOpts {
                        min_match,
                        ..ImageOpts::default()
                    }),
                }
            }
            _ => {}
        }
    }

    if image_data.image.is_none() {
        return Err(Generic(
            "An image is required but was not provided".to_string(),
        ));
    }

    Ok(image_data)
}

/// Extract image and details from multipart form data.
pub async fn extract_enroll_data(mut multipart: Multipart) -> WResult<EnrollData> {
    let mut enroll_data = EnrollData {
        image: None,
        details: None,
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                match bytes.len() {
                    0 => enroll_data.image = None,
                    _ => enroll_data.image = Some(general_purpose::STANDARD.encode(bytes)),
                };
            }
            "details" => {
                debug!("received details for enrollment");
                let details = field.text().await.map_err(|x| Generic(x.to_string()))?;
                let enroll_det = serde_json::from_str(&details);
                debug!("{:?}", &enroll_det);
                if let Ok(d) = enroll_det {
                    enroll_data.details = Some(d);
                }
            }
            _ => {}
        }
    }

    match enroll_data.borrow() {
        EnrollData {
            image: Some(_),
            details: None,
        } => Err(Generic(
            "You need to provide details to know who this person is!".to_string(),
        )),
        EnrollData {
            image: None,
            details: None,
        } => Err(Generic(
            "Nothing was provided! What would we be enrolling?".to_string(),
        )),
        _ => Ok(enroll_data),
    }
}

pub async fn extract_new_profile_req(mut multipart: Multipart) -> WResult<NewProfileRequest> {
    let mut image: Option<String> = None;
    let mut profile: Option<NewProfileRequest> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                image = Some(general_purpose::STANDARD.encode(bytes));
            }
            "profile" => {
                info!("got new profile request");
                let p_txt = field.text().await.map_err(|x| Generic(x.to_string()))?;
                let p = serde_json::from_str(&p_txt);
                match p {
                    Ok(d) => {
                        profile = Some(d);
                    }
                    Err(e) => {
                        let emsg = format!("couldn't parse profile: {}", e);
                        return Err(Generic(emsg));
                    }
                }
            }
            _ => {}
        }
    }

    if image.is_none() {
        return Err(Generic(
            "Creating a profile requires an image which was not provided".to_string(),
        ));
    }

    match profile {
        Some(mut p) => {
            p.image = image;
            Ok(p)
        }
        None => Err(Generic(
            "Creating a profile requires personal info which was not provided".to_string(),
        )),
    }
}
