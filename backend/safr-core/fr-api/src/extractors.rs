//! Multipart extraction helpers for the HTTP API.
//!
//! The public API accepts a mix of JSON and multipart requests. These helpers normalize multipart
//! payloads into typed request structs used by handlers and `libfr`:
//! - image uploads may be sent as raw bytes or base64-encoded text
//! - multipart JSON fields such as `opts`, `details`, and `profile` are deserialized in-place
//! - image bytes are validated up front so handlers can rely on downstream invariants

use axum::extract::multipart::Multipart;
use base64::{Engine as _, engine::general_purpose};
use bytes::Bytes;
use image::ImageFormat;
use libfr::tpass::types::NewProfileRequest;
use libfr::types::EnrollData;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::errors::AppError;
use crate::errors::AppError::Generic;

type WResult<T> = Result<T, AppError>;

/// Optional recognition controls accepted by multipart recognition-style endpoints.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RecognizeOpts {
    pub top_matches: u8,
    pub include_detected_faces: bool,
    pub on_match: String,
    pub min_match: f32,
    pub rec_location: String,
    pub include_details: bool,
}

impl Default for RecognizeOpts {
    fn default() -> Self {
        Self {
            top_matches: 1,
            include_detected_faces: false,
            on_match: "id_only".to_string(),
            min_match: 0.90,
            rec_location: "".to_string(),
            include_details: false,
        }
    }
}

/// Parsed multipart payload for endpoints that accept an image plus recognition options.
#[derive(Debug, Clone)]
pub struct RecognizeFormData {
    pub image: Option<Bytes>,
    pub opts: Option<RecognizeOpts>,
}

/// Parsed multipart payload for the add-face endpoint.
#[derive(Debug, Clone)]
pub struct AddFaceFormData {
    pub image: Option<Bytes>,
    pub fr_id: String,
}

/// Parsed multipart payload for create-profile plus enrollment flows.
#[derive(Debug)]
pub struct NewProfileEnrollData {
    pub profile: NewProfileRequest,
    pub image: Bytes,
}

/// Extract image bytes and optional recognition settings from multipart form data.
pub async fn extract_image_data(
    mut multipart: Multipart,
    min_match: f32,
) -> WResult<RecognizeFormData> {
    let mut image_data = RecognizeFormData { image: None, opts: None };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                //let content_type = field.content_type().map(|item| item.to_string());
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                image_data.image = parse_image_field(bytes)?;
            }

            "opts" => {
                let jval = field.text().await.map_err(|x| Generic(x.to_string()))?;
                let p_res = serde_json::from_str(jval.as_str());
                image_data.opts = match p_res {
                    Ok(opts) => Some(opts),
                    Err(e) => {
                        error!("{:?}", e);
                        Some(RecognizeOpts { min_match, ..RecognizeOpts::default() })
                    }
                }
            }
            _ => {}
        }
    }

    if image_data.image.is_none() {
        return Err(Generic("An image is required but was not provided".to_string()));
    }

    Ok(image_data)
}

/// Extract the image and target identity id used by the add-face route.
pub async fn extract_add_face_form_data(mut multipart: Multipart) -> WResult<AddFaceFormData> {
    //    let mut image_data = RecognizeFormData { image: None, opts: None };
    let mut face_req = AddFaceFormData { image: None, fr_id: "".to_string() };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                //let content_type = field.content_type().map(|item| item.to_string());
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                face_req.image = parse_image_field(bytes)?;
            }

            "fr_id" => {
                let val = field.text().await.map_err(|x| Generic(x.to_string()))?;
                face_req.fr_id = val;
            }
            _ => {}
        }
    }

    if face_req.image.is_none() {
        return Err(Generic("Image not sent!".to_string()));
    }

    Ok(face_req)
}

/// Extract enrollment image bytes plus `EnrollDetails` from multipart form data.
pub async fn extract_enroll_data(mut multipart: Multipart) -> WResult<EnrollData> {
    let mut enroll_data = EnrollData { image: None, details: None };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                //let content_type = field.content_type().map(|item| item.to_string());
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                enroll_data.image = parse_image_field(bytes)?;
            }
            "details" => {
                debug!("received details for enrollment");
                //TODO: make sure error is returned if details are NONE.
                let details = field.text().await.map_err(|x| Generic(x.to_string()))?;

                let enroll_det = serde_json::from_str(&details);
                debug!("{:?}", &enroll_det);
                enroll_data.details = enroll_det.ok();
            }
            _ => {}
        }
    }

    match (&enroll_data.image, &enroll_data.details) {
        (Some(_), None) => Err(Generic("Missing enrollment details".to_string())),
        (None, None) => {
            Err(Generic("Nothing was provided! What would we be enrolling?".to_string()))
        }
        _ => Ok(enroll_data),
    }
}

/// Extract a new TPass profile request and its associated enrollment image.
pub async fn extract_new_profile_req(mut multipart: Multipart) -> WResult<NewProfileEnrollData> {
    let mut image: Option<Bytes> = None;
    let mut profile: Option<NewProfileRequest> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Generic(format!("invalid multipart payload: {}", e)))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                //let content_type = field.content_type().map(|item| item.to_string());
                let bytes = field.bytes().await.map_err(|x| Generic(x.to_string()))?;
                image = parse_image_field(bytes)?;
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

    let image = image.ok_or_else(|| {
        Generic("Creating a profile requires an image which was not provided".to_string())
    })?;

    match profile {
        Some(mut profile) => {
            profile.image = Some(general_purpose::STANDARD.encode(&image));
            Ok(NewProfileEnrollData { profile, image })
        }
        None => Err(Generic(
            "Creating a profile requires personal info which was not provided".to_string(),
        )),
    }
}

/// Accept raw image bytes or base64 text and validate the resulting image format.
fn parse_image_field(raw_bytes: Bytes) -> WResult<Option<Bytes>> {
    if raw_bytes.is_empty() {
        return Ok(None);
    }

    let bytes = if let Ok(text) = std::str::from_utf8(&raw_bytes) {
        match decode_base64_image(text) {
            Ok(decoded) => decoded,
            Err(_) => raw_bytes,
        }
    } else {
        raw_bytes
    };

    guess_image_format(&bytes)?;

    Ok(Some(bytes))
}

/// Decode a base64 image payload, including `data:` URL style prefixes.
fn decode_base64_image(input: &str) -> WResult<Bytes> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return Err(Generic("image field was empty".to_string()));
    }

    let payload = if let Some((prefix, value)) = cleaned.split_once(',') {
        let prefix = prefix.trim().to_ascii_lowercase();

        if prefix.starts_with("data:") && prefix.contains(";base64") {
            value.trim()
        } else {
            cleaned
        }
    } else {
        cleaned
    };

    for engine in [
        &general_purpose::STANDARD,
        &general_purpose::STANDARD_NO_PAD,
        &general_purpose::URL_SAFE,
        &general_purpose::URL_SAFE_NO_PAD,
    ] {
        if let Ok(decoded) = engine.decode(payload) {
            return Ok(Bytes::from(decoded));
        }
    }

    Err(Generic("image field was text but was not valid base64".to_string()))
}

/// Validate that the payload is a recognizable image format.
fn guess_image_format(bytes: &[u8]) -> WResult<ImageFormat> {
    image::guess_format(bytes)
        .map_err(|_| Generic("uploaded data is not a recognized image format".to_string()))
}

//check if the raw_bytes based in are the actual image bytes or base64 encoded
//and either convert the base64 encoding or return the raw_bytes unchanged.
// fn parse_image_field(raw_bytes: Bytes, content_type: Option<&str>) -> WResult<Option<Bytes>> {
//     if raw_bytes.is_empty() {
//         return Ok(None);
//     }

//     //NOTE: the client must name the image portion of the upload, "image" or no werky, jerky
//     if content_type.is_some_and(|item| item.starts_with("image")) {
//         return Ok(Some(raw_bytes));
//     }

//     match std::str::from_utf8(&raw_bytes) {
//         Ok(text) => decode_base64_image(text).map(Some),
//         Err(_) => Ok(Some(raw_bytes)),
//     }
// }

// //NOTE: how long do we plan to keep the base64 version of images. binary should at least
// //be the preferred default. remember base64 is 33% larger.
// fn decode_base64_image(input: &str) -> WResult<Bytes> {
//     let cleaned = input.trim();
//     if cleaned.is_empty() {
//         return Err(Generic("image field was empty".to_string()));
//     }

//     let payload = cleaned
//         .split_once(',')
//         .filter(|(prefix, _)| {
//             prefix.to_ascii_lowercase().contains(";base64")
//                 || prefix.to_ascii_lowercase().starts_with("data:")
//         })
//         .map_or(cleaned, |(_, value)| value)
//         .trim();

//     //NOTE: this seems a bit extra but we'll leave for now since it doesn't hurt to be explicit.

//     for engine in [
//         &general_purpose::STANDARD,
//         &general_purpose::STANDARD_NO_PAD,
//         &general_purpose::URL_SAFE,
//         &general_purpose::URL_SAFE_NO_PAD,
//     ] {
//         if let Ok(decoded) = engine.decode(payload) {
//             return Ok(Bytes::from(decoded));
//         }
//     }

//     Err(Generic("image field was text but was not valid base64".to_string()))
// }
