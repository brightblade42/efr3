use chrono::{DateTime, SecondsFormat, Utc};

use crate::identity_grpc::identity;
use crate::types::{
    AddFaceInput, AddFaceResponse, CreateIdentitiesInput, CreateIdentitiesResponse,
    DeleteFaceInput, DeleteFaceResponse, Embedding, FaceInfo, GetFacesInput, GetFacesResponse,
    GetIdentitiesInput, Identities, Identity, IdentityMatch, LookupIdentities, LookupIdentity,
};

const DEFAULT_SCALING_FACTOR: f32 = 2.0;
const DEFAULT_BUCKETS_LIMIT: i64 = 32;

pub(crate) fn lookup_request(embeddings: Vec<Embedding>, limit: i32) -> identity::LookupRequest {
    identity::LookupRequest {
        group_ids: vec![],
        embeddings: embeddings.into_iter().map(Into::into).collect(),
        limit,
        model: String::new(),
        scaling_factor: DEFAULT_SCALING_FACTOR,
        buckets_limit: DEFAULT_BUCKETS_LIMIT,
    }
}

pub(crate) fn get_faces_request(req: GetFacesInput, page_size: u32) -> identity::GetFacesRequest {
    identity::GetFacesRequest {
        identity_id: req.fr_id,
        page_token: String::new(),
        page_size: page_size as i32,
    }
}

pub(crate) fn delete_identities_request(
    id: Option<String>,
    external_id: Option<String>,
) -> identity::DeleteIdentitiesRequest {
    identity::DeleteIdentitiesRequest {
        ids: id.into_iter().collect(),
        external_ids: external_id.into_iter().collect(),
    }
}

impl From<Embedding> for identity::Embedding {
    fn from(embedding: Embedding) -> Self {
        Self {
            embedding: embedding.embedding,
        }
    }
}

impl From<GetIdentitiesInput> for identity::GetIdentitiesRequest {
    fn from(req: GetIdentitiesInput) -> Self {
        Self {
            group_ids: req.group_ids.unwrap_or_default(),
            page_token: req.page_token.unwrap_or_default(),
            page_size: req.page_size as i32,
        }
    }
}

impl From<CreateIdentitiesInput> for identity::CreateIdentitiesRequest {
    fn from(req: CreateIdentitiesInput) -> Self {
        Self {
            group_ids: req.group_ids.unwrap_or_default(),
            embeddings: req.embeddings.into_iter().map(Into::into).collect(),
            threshold: req.threshold,
            model: String::new(),
            qualities: req.qualities,
            external_ids: req.external_ids.unwrap_or_default(),
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
            options: vec![],
        }
    }
}

impl From<AddFaceInput> for identity::AddFacesRequest {
    fn from(req: AddFaceInput) -> Self {
        Self {
            identity_id: req.identity_id,
            embeddings: req.embeddings.into_iter().map(Into::into).collect(),
            threshold: req.threshold,
            model: String::new(),
            qualities: req.qualities,
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
            flush: Some(true),
        }
    }
}

impl From<&DeleteFaceInput> for identity::DeleteFacesRequest {
    fn from(req: &DeleteFaceInput) -> Self {
        Self {
            identity_id: req.fr_id.clone(),
            face_ids: vec![req.face_id.clone()],
        }
    }
}

impl From<identity::GetIdentitiesResponse> for Identities {
    fn from(response: identity::GetIdentitiesResponse) -> Self {
        Self {
            identities: response.identities.into_iter().map(Into::into).collect(),
            next_page_token: response.next_page_token,
            total_size: response.total_size.max(0) as u64,
        }
    }
}

impl From<identity::CreateIdentitiesResponse> for CreateIdentitiesResponse {
    fn from(response: identity::CreateIdentitiesResponse) -> Self {
        Self {
            identities: response.identities.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<identity::AddFacesResponse> for AddFaceResponse {
    fn from(response: identity::AddFacesResponse) -> Self {
        Self {
            faces: response.faces.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<identity::DeleteFacesResponse> for DeleteFaceResponse {
    fn from(response: identity::DeleteFacesResponse) -> Self {
        Self {
            rows_affected: response.rows_affected,
        }
    }
}

impl From<identity::GetFacesResponse> for GetFacesResponse {
    fn from(response: identity::GetFacesResponse) -> Self {
        Self {
            faces: response.faces.into_iter().map(Into::into).collect(),
            next_page_token: response.next_page_token,
            total_size: response.total_size,
        }
    }
}

impl From<identity::LookupResponse> for LookupIdentities {
    fn from(response: identity::LookupResponse) -> Self {
        Self {
            lookup_identities: response
                .lookup_identities
                .into_iter()
                .map(|item| LookupIdentity {
                    matches: item
                        .matches
                        .into_iter()
                        .map(|item| IdentityMatch {
                            identity: item.identity.map(Into::into).unwrap_or_else(empty_identity),
                            score: item.score,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl From<identity::Identity> for Identity {
    fn from(identity: identity::Identity) -> Self {
        Self {
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
}

impl From<identity::Face> for FaceInfo {
    fn from(face: identity::Face) -> Self {
        Self {
            id: face.id,
            identity_id: face.identity_id,
            created_at: timestamp_to_rfc3339(face.created_at),
            model: face.model,
            quality: face.quality,
        }
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
