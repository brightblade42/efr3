use chrono::{DateTime, SecondsFormat, Utc};

use crate::identity_grpc::identity;
use crate::types::{
    AddFaceInput, AddFaceResponse, CreateIdentitiesInput, CreateIdentitiesResponse,
    DeleteFaceInput, DeleteFaceResponse, Embedding, FaceInfo, GetFacesInput, GetFacesResponse,
    GetIdentitiesInput, Identities, Identity, IdentityMatch, LookupIdentities, LookupIdentity,
};

const DEFAULT_SCALING_FACTOR: f32 = 2.0;
const DEFAULT_BUCKETS_LIMIT: i64 = 32;

pub(crate) fn to_get_identities_request(req: GetIdentitiesInput) -> identity::GetIdentitiesRequest {
    identity::GetIdentitiesRequest {
        group_ids: req.group_ids.unwrap_or_default(),
        page_token: req.page_token.unwrap_or_default(),
        page_size: req.page_size as i32,
    }
}

pub(crate) fn to_create_identities_request(
    req: CreateIdentitiesInput,
) -> identity::CreateIdentitiesRequest {
    identity::CreateIdentitiesRequest {
        group_ids: req.group_ids.unwrap_or_default(),
        embeddings: req.embeddings.into_iter().map(to_proto_embedding).collect(),
        threshold: req.threshold,
        model: String::new(),
        qualities: req.qualities,
        external_ids: req.external_ids.unwrap_or_default(),
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
        options: vec![],
    }
}

pub(crate) fn to_lookup_request(embeddings: Vec<Embedding>, limit: i32) -> identity::LookupRequest {
    identity::LookupRequest {
        group_ids: vec![],
        embeddings: embeddings.into_iter().map(to_proto_embedding).collect(),
        limit,
        model: String::new(),
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
    }
}

pub(crate) fn to_add_face_request(req: AddFaceInput) -> identity::AddFacesRequest {
    identity::AddFacesRequest {
        identity_id: req.identity_id,
        embeddings: req.embeddings.into_iter().map(to_proto_embedding).collect(),
        threshold: req.threshold,
        model: String::new(),
        qualities: req.qualities,
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
        flush: Some(true),
    }
}

pub(crate) fn to_delete_faces_request(req: &DeleteFaceInput) -> identity::DeleteFacesRequest {
    identity::DeleteFacesRequest {
        identity_id: req.fr_id.clone(),
        face_ids: vec![req.face_id.clone()],
    }
}

pub(crate) fn to_get_faces_request(
    req: GetFacesInput,
    page_size: u32,
) -> identity::GetFacesRequest {
    identity::GetFacesRequest {
        identity_id: req.fr_id,
        page_token: String::new(),
        page_size: page_size as i32,
    }
}

pub(crate) fn to_delete_identities_request(
    id: Option<String>,
    external_id: Option<String>,
) -> identity::DeleteIdentitiesRequest {
    identity::DeleteIdentitiesRequest {
        ids: id.into_iter().collect(),
        external_ids: external_id.into_iter().collect(),
    }
}

pub(crate) fn to_identities(response: identity::GetIdentitiesResponse) -> Identities {
    Identities {
        identities: response.identities.into_iter().map(to_identity).collect(),
        next_page_token: response.next_page_token,
        total_size: response.total_size.max(0) as u64,
    }
}

pub(crate) fn to_create_identities_response(
    response: identity::CreateIdentitiesResponse,
) -> CreateIdentitiesResponse {
    CreateIdentitiesResponse {
        identities: response.identities.into_iter().map(to_identity).collect(),
    }
}

pub(crate) fn to_add_face_response(response: identity::AddFacesResponse) -> AddFaceResponse {
    AddFaceResponse {
        faces: response.faces.into_iter().map(to_face_info).collect(),
    }
}

pub(crate) fn to_delete_face_response(
    response: identity::DeleteFacesResponse,
) -> DeleteFaceResponse {
    DeleteFaceResponse {
        rows_affected: response.rows_affected,
    }
}

pub(crate) fn to_get_faces_response(response: identity::GetFacesResponse) -> GetFacesResponse {
    GetFacesResponse {
        faces: response.faces.into_iter().map(to_face_info).collect(),
        next_page_token: response.next_page_token,
        total_size: response.total_size,
    }
}

pub(crate) fn to_lookup_identities(response: identity::LookupResponse) -> LookupIdentities {
    LookupIdentities {
        lookup_identities: response
            .lookup_identities
            .into_iter()
            .map(|item| LookupIdentity {
                matches: item
                    .matches
                    .into_iter()
                    .map(|item| IdentityMatch {
                        identity: item
                            .identity
                            .map(to_identity)
                            .unwrap_or_else(empty_identity),
                        score: item.score,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn to_proto_embedding(embedding: Embedding) -> identity::Embedding {
    identity::Embedding {
        embedding: embedding.embedding,
    }
}

fn to_identity(identity: identity::Identity) -> Identity {
    Identity {
        id: identity.id,
        created_at: timestamp_to_rfc3339(identity.created_at),
        external_id: if identity.external_id.is_empty() {
            None
        } else {
            Some(identity.external_id)
        },
        updated_at: timestamp_to_rfc3339(identity.updated_at),
        group_ids: if identity.group_ids.is_empty() {
            None
        } else {
            Some(identity.group_ids)
        },
    }
}

fn to_face_info(face: identity::Face) -> FaceInfo {
    FaceInfo {
        id: face.id,
        identity_id: face.identity_id,
        created_at: timestamp_to_rfc3339(face.created_at),
        model: face.model,
        quality: face.quality,
    }
}

fn empty_identity() -> Identity {
    Identity {
        id: String::new(),
        created_at: String::new(),
        external_id: None,
        updated_at: String::new(),
        group_ids: None,
    }
}

fn timestamp_to_rfc3339(timestamp: Option<prost_types::Timestamp>) -> String {
    let Some(timestamp) = timestamp else {
        return String::new();
    };

    let Some(datetime) = DateTime::<Utc>::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
    else {
        return String::new();
    };

    datetime.to_rfc3339_opts(SecondsFormat::Micros, true)
}
