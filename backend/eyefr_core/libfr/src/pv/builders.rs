use bytes::Bytes;

use crate::pv::pv_grpc::{identity_grpc::identity, proc_grpc::processor};
use crate::types::Face;

pub(super) const DEFAULT_SCALING_FACTOR: f32 = 2.0;
pub(super) const DEFAULT_BUCKETS_LIMIT: i64 = 32;

pub(super) fn build_process_image_request(image: Bytes) -> processor::ProcessFullImageRequest {
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

pub(super) fn liveness_process_full_image_request(
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

pub(super) fn build_lookup_request(
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

pub(super) fn build_add_faces_request(
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

pub(super) fn build_delete_faces_request(
    fr_id: &str,
    face_ids: Vec<String>,
) -> identity::DeleteFacesRequest {
    identity::DeleteFacesRequest { identity_id: fr_id.to_string(), face_ids }
}

pub(super) fn build_ident_request(
    face: &Face,
    dupe_match: f32,
    ext_id: &str,
) -> identity::CreateIdentitiesRequest {
    let emb = face.template.clone().unwrap().embedding;

    identity::CreateIdentitiesRequest {
        group_ids: vec![],
        embeddings: vec![identity::Embedding { embedding: emb }],
        threshold: dupe_match,
        model: String::new(),
        qualities: vec![face.quality.unwrap_or(0.0)],
        external_ids: vec![ext_id.to_string()],
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
        options: vec![],
    }
}

fn default_liveness_validness_parameters()
-> processor::process_full_image_request::LivenessValidnessParameters {
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
