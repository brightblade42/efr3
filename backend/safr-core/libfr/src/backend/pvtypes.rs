use bytes::Bytes;
use chrono::{DateTime, SecondsFormat, Utc};
use libpv::identity_grpc::identity;
use libpv::proc_grpc::processor;

use crate::{utils, Face, PossibleMatch, Template};

pub const DEFAULT_SCALING_FACTOR: f32 = 2.0;
pub const DEFAULT_BUCKETS_LIMIT: i64 = 32;

pub(crate) fn build_process_image_request(image: Bytes) -> processor::ProcessFullImageRequest {
    use processor::process_full_image_request::Options;

    processor::ProcessFullImageRequest {
        image: image.to_vec(),
        outputs: vec![
            Options::BoundingBox as i32,
            Options::Embedding as i32,
            Options::Quality as i32,
            Options::Mask as i32,
        ],
        find_most_prominent_face: true,
        scoring_mode: processor::ScoringMode::Auto as i32,
        image_source: processor::ImageSource::Unknown as i32,
        liveness_validness_parameters: None,
        ages_v2_validness_parameters: None,
        deepfake_validness_parameters: None,
    }
}

pub(crate) fn liveness_process_full_image_request(
    image: Bytes,
) -> processor::ProcessFullImageRequest {
    use processor::process_full_image_request::Options;

    processor::ProcessFullImageRequest {
        image: image.to_vec(),
        outputs: vec![
            Options::BoundingBox as i32,
            Options::Quality as i32,
            Options::Liveness as i32,
            Options::LivenessValidness as i32,
        ],
        find_most_prominent_face: true,
        scoring_mode: processor::ScoringMode::Auto as i32,
        image_source: processor::ImageSource::Webcam as i32,
        liveness_validness_parameters: Some(default_liveness_validness_parameters()),
        ages_v2_validness_parameters: None,
        deepfake_validness_parameters: None,
    }
}

pub(crate) fn build_lookup_request(
    processed: processor::ProcessFullImageResponse,
    limit: i32,
) -> Option<(Vec<processor::Face>, identity::LookupRequest)> {
    let mut faces_with_embeddings = Vec::new();
    let mut embeddings = Vec::new();

    for face in processed.faces {
        if face.embedding.is_empty() {
            continue;
        }
        embeddings.push(identity::Embedding { embedding: face.embedding.clone() });
        faces_with_embeddings.push(face);
    }

    if faces_with_embeddings.is_empty() {
        return None;
    }

    Some((
        faces_with_embeddings,
        identity::LookupRequest {
            group_ids: vec![],
            embeddings,
            limit,
            model: String::new(),
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
        },
    ))
}

pub(crate) fn add_faces_request_from_processed(
    processed: processor::ProcessFullImageResponse,
    identity_id: String,
    threshold: f32,
) -> identity::AddFacesRequest {
    let mut embeddings = Vec::new();
    let mut qualities = Vec::new();

    for face in processed.faces {
        if face.embedding.is_empty() {
            continue;
        }

        embeddings.push(identity::Embedding { embedding: face.embedding });
        if face.quality.is_finite() {
            qualities.push(face.quality);
        }
    }

    identity::AddFacesRequest {
        identity_id,
        embeddings,
        threshold,
        model: String::new(),
        qualities,
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
        flush: Some(true),
    }
}

pub(crate) fn get_faces_request(fr_id: &str) -> identity::GetFacesRequest {
    identity::GetFacesRequest {
        identity_id: fr_id.to_string(),
        page_token: String::new(),
        page_size: 100,
    }
}

pub(crate) fn delete_faces_request(
    fr_id: &str,
    face_ids: Vec<String>,
) -> identity::DeleteFacesRequest {
    identity::DeleteFacesRequest { identity_id: fr_id.to_string(), face_ids: face_ids }
}

// pub(crate) fn to_add_face_result(response: identity::AddFacesResponse) -> AddFaceResult {
//     AddFaceResult { faces: response.faces.into_iter().map(to_enrollment_face_info).collect() }
// }

// pub(crate) fn to_get_face_info_result(response: identity::GetFacesResponse) -> GetFaceInfoResult {
//     GetFaceInfoResult {
//         faces: response.faces.into_iter().map(to_enrollment_face_info).collect(),
//         next_page_token: response.next_page_token,
//         total_size: response.total_size,
//     }
// }

// pub(crate) fn to_enrollment_face_info(face: identity::Face) -> EnrollmentFaceInfo {
//     EnrollmentFaceInfo {
//         id: face.id,
//         identity_id: face.identity_id,
//         created_at: timestamp_to_rfc3339(face.created_at),
//         model: face.model,
//         quality: face.quality,
//     }
// }

pub(crate) fn possible_matches_from_lookup(
    lookup: &identity::LookupIdentity,
) -> Vec<PossibleMatch> {
    let mut possible_matches: Vec<PossibleMatch> = lookup
        .matches
        .iter()
        .filter_map(|match_item| {
            let identity = match_item.identity.as_ref()?;
            let score = utils::roundf32(match_item.score, 5);
            let mut possible_match = PossibleMatch::new(identity.id.clone(), score);
            possible_match.ext_id = identity.external_id.clone();
            Some(possible_match)
        })
        .collect();

    if possible_matches.len() > 1 {
        possible_matches.sort_by(|a, b| {
            a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal).reverse()
        });
    }

    possible_matches
}

impl From<processor::Face> for Face {
    fn from(pv_face: processor::Face) -> Self {
        let bbox = pv_face.bounding_box.as_ref().map(|bb| crate::BoundingBox {
            origin: crate::Point {
                x: bb.origin.as_ref().map_or(0.0, |point| point.x.floor()),
                y: bb.origin.as_ref().map_or(0.0, |point| point.y.floor()),
            },
            width: bb.width.round(),
            height: bb.height.round(),
        });

        let liveness = to_liveness(pv_face.liveness.as_ref(), pv_face.liveness_validness.as_ref());

        let template =
            (!pv_face.embedding.is_empty()).then(|| Template { embedding: pv_face.embedding });

        Self {
            bbox,
            acceptability: Some(pv_face.acceptability),
            quality: Some(pv_face.quality),
            mask: Some(pv_face.mask),
            template,
            liveness,
        }
    }
}

impl From<&processor::Face> for Face {
    fn from(pv_face: &processor::Face) -> Self {
        let bbox = pv_face.bounding_box.as_ref().map(|bb| crate::BoundingBox {
            origin: crate::Point {
                x: bb.origin.as_ref().map_or(0.0, |point| point.x.floor()),
                y: bb.origin.as_ref().map_or(0.0, |point| point.y.floor()),
            },
            width: bb.width.round(),
            height: bb.height.round(),
        });

        let liveness = to_liveness(pv_face.liveness.as_ref(), pv_face.liveness_validness.as_ref());

        Self {
            bbox,
            acceptability: Some(pv_face.acceptability),
            quality: Some(pv_face.quality),
            mask: Some(pv_face.mask),
            template: None,
            liveness,
        }
    }
}
fn to_liveness(
    liveness: Option<&processor::Liveness>,
    validness: Option<&processor::Validness>,
) -> Option<crate::Liveness> {
    // 1. Early return if liveness is None.
    let liveness = liveness?;

    let is_valid = validness.map_or(false, |v| v.is_valid);

    // 3. Process feedback without double-evaluating validness
    let feedback = validness
        .map(|v| v.feedback.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|code| {
            processor::validness::Feedback::try_from(*code)
                .unwrap_or(processor::validness::Feedback::Unknown)
                .as_str_name()
                .to_string()
        })
        .collect();

    let is_live = is_valid && liveness.liveness_probability > 0.5;

    // Return the final mapped struct wrapped in Some
    Some(crate::Liveness { is_live, feedback, score: liveness.liveness_probability })
}

fn default_liveness_validness_parameters(
) -> processor::process_full_image_request::LivenessValidnessParameters {
    let mut params = processor::process_full_image_request::LivenessValidnessParameters::default();
    params.min_face_sharpness = Some(0.15);
    params.min_face_quality = Some(0.5);
    params.min_face_acceptability = Some(0.15);
    params.min_face_frontality = Some(70);
    params.max_face_mask_probability = Some(0.5);
    params.image_illumination_control = Some(50);
    params.max_face_size_pct = Some(0.72);
    params.image_boundary_width_pct = Some(0.8);
    params.image_boundary_height_pct = Some(0.8);
    params.min_face_size = Some(100);
    params.max_face_roll_angle = Some(45);
    params.fail_fast = Some(true);
    params
}

pub(crate) fn timestamp_to_rfc3339(timestamp: Option<prost_types::Timestamp>) -> String {
    let Some(timestamp) = timestamp else {
        return String::new();
    };

    let Some(datetime) = DateTime::<Utc>::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
    else {
        return String::new();
    };

    datetime.to_rfc3339_opts(SecondsFormat::Micros, true)
}
