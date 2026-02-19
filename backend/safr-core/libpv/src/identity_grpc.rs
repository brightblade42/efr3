use chrono::{DateTime, SecondsFormat, Utc};
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::types::{
    AddFaceRequest, AddFaceResponse, CreateIdentitiesRequest, CreateIdentitiesResponse,
    DeleteFaceRequest, DeleteFaceResponse, DeleteIdentitiesRequest, Embedding, Face, FaceInfo,
    GetFacesRequest, GetFacesResponse, GetIdentitiesRequest, Identities, Identity, IdentityMatch,
    LookupIdentities, LookupIdentity, LookupRequest, LookupResponse,
};

type PVResult<T> = Result<T, PVApiError>;
type PVResultMany<T> = PVResult<Vec<PVResult<T>>>;

const DEFAULT_PAGE_SIZE: u32 = 100;
const DEFAULT_SCALING_FACTOR: f32 = 2.0;
const DEFAULT_BUCKETS_LIMIT: i64 = 32;

pub mod identity {
    tonic::include_proto!("identity.v7");
}

#[derive(Clone)]
pub struct PVIdentityGrpcApi {
    endpoint: String,
}

impl PVIdentityGrpcApi {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint: normalize_endpoint(endpoint),
        }
    }

    pub async fn get_identities(&self, req: Option<GetIdentitiesRequest>) -> PVResult<Identities> {
        let mut client = self.identity_client().await?;
        let req = req.unwrap_or(GetIdentitiesRequest {
            page_size: DEFAULT_PAGE_SIZE,
            page_token: Some(String::new()),
            group_ids: None,
        });

        let grpc_req = identity::GetIdentitiesRequest {
            group_ids: req.group_ids.unwrap_or_default(),
            page_token: req.page_token.unwrap_or_default(),
            page_size: req.page_size as i32,
        };

        let response = client
            .get_identities(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(Identities {
            identities: response.identities.into_iter().map(to_identity).collect(),
            next_page_token: response.next_page_token,
            total_size: response.total_size.max(0) as u64,
        })
    }

    pub async fn create_identities(
        &self,
        req: CreateIdentitiesRequest,
    ) -> PVResult<CreateIdentitiesResponse> {
        let mut client = self.identity_client().await?;

        let grpc_req = identity::CreateIdentitiesRequest {
            group_ids: req.group_ids.unwrap_or_default(),
            embeddings: req.embeddings.into_iter().map(to_proto_embedding).collect(),
            threshold: req.threshold,
            model: String::new(),
            qualities: req.qualities,
            external_ids: req.external_ids.unwrap_or_default(),
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
            options: vec![],
        };

        let response = client
            .create_identities(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(CreateIdentitiesResponse {
            identities: response.identities.into_iter().map(to_identity).collect(),
        })
    }

    pub async fn delete_identities(
        &self,
        delete_req: Option<DeleteIdentitiesRequest>,
    ) -> PVResultMany<String> {
        let delete_targets: Vec<(Option<String>, Option<String>, String)> = match delete_req {
            None => {
                let req = Some(GetIdentitiesRequest {
                    page_size: 100000,
                    page_token: Some(String::new()),
                    group_ids: None,
                });

                self.get_identities(req)
                    .await?
                    .identities
                    .into_iter()
                    .map(|item| {
                        let id = item.id;
                        (Some(id.clone()), None, id)
                    })
                    .collect()
            }
            Some(req) => {
                let external_ids = req.external_ids.unwrap_or_default();

                if req.ids.is_empty() {
                    external_ids
                        .into_iter()
                        .map(|external_id| {
                            let label = format!("external_id={}", external_id);
                            (None, Some(external_id), label)
                        })
                        .collect()
                } else {
                    req.ids
                        .into_iter()
                        .enumerate()
                        .map(|(idx, id)| {
                            let external_id = external_ids.get(idx).cloned();
                            let label = external_id
                                .as_ref()
                                .map_or_else(|| id.clone(), |item| format!("{} ({})", id, item));
                            (Some(id), external_id, label)
                        })
                        .collect()
                }
            }
        };

        if delete_targets.is_empty() {
            return Ok(vec![]);
        }

        let mut client = self.identity_client().await?;
        let mut results: Vec<PVResult<String>> = Vec::with_capacity(delete_targets.len());

        for (id, external_id, label) in delete_targets {
            let grpc_req = identity::DeleteIdentitiesRequest {
                ids: id.into_iter().collect(),
                external_ids: external_id.into_iter().collect(),
            };

            match client.delete_identities(Request::new(grpc_req)).await {
                Ok(response) => {
                    if response.into_inner().rows_affected > 0 {
                        results.push(Ok(label));
                    } else {
                        results.push(Err(PVApiError::with_code(
                            404,
                            &format!("identity '{}' was not deleted", label),
                        )));
                    }
                }
                Err(err) => results.push(Err(PVApiError::from(err))),
            }
        }

        Ok(results)
    }

    pub async fn lookup_single(&self, embedding: Embedding) -> PVResult<LookupIdentities> {
        let mut client = self.identity_client().await?;

        let req = identity::LookupRequest {
            group_ids: vec![],
            embeddings: vec![to_proto_embedding(embedding)],
            limit: 1,
            model: String::new(),
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
        };

        let response = client.lookup(Request::new(req)).await?.into_inner();
        Ok(to_lookup_identities(response))
    }

    pub async fn lookup<I: Into<LookupRequest>>(&self, req: I) -> PVResultMany<LookupResponse> {
        let req: LookupRequest = req.into();
        let faces = req
            .faces
            .ok_or_else(|| PVApiError::with_code(500, "lookup: There were no faces provided"))?;
        let limit = req.limit;

        let mut client = self.identity_client().await?;
        let mut results: Vec<PVResult<LookupResponse>> = Vec::with_capacity(faces.len());

        for face in faces {
            let embedding = match face.embedding.clone() {
                Some(embedding) => embedding,
                None => {
                    results.push(Err(PVApiError::with_code(
                        500,
                        "lookup: face was provided without an embedding",
                    )));
                    continue;
                }
            };

            let lookup_req = identity::LookupRequest {
                group_ids: vec![],
                embeddings: vec![to_proto_embedding(Embedding { embedding })],
                limit,
                model: String::new(),
                scaling_factor: DEFAULT_SCALING_FACTOR,
                buckets_limit: DEFAULT_BUCKETS_LIMIT,
            };

            match client.lookup(Request::new(lookup_req)).await {
                Ok(response) => {
                    results.push(Ok(LookupResponse {
                        face: Face {
                            embedding: None,
                            ..face
                        },
                        identities: to_lookup_identities(response.into_inner()),
                    }));
                }
                Err(err) => results.push(Err(PVApiError::from(err))),
            }
        }

        Ok(results)
    }

    pub async fn add_face(&self, req: AddFaceRequest) -> PVResult<AddFaceResponse> {
        let mut client = self.identity_client().await?;

        let grpc_req = identity::AddFacesRequest {
            identity_id: req.identity_id,
            embeddings: req.embeddings.into_iter().map(to_proto_embedding).collect(),
            threshold: req.threshold,
            model: String::new(),
            qualities: req.qualities,
            scaling_factor: DEFAULT_SCALING_FACTOR,
            buckets_limit: DEFAULT_BUCKETS_LIMIT,
            flush: Some(true),
        };

        let response = client.add_faces(Request::new(grpc_req)).await?.into_inner();
        Ok(AddFaceResponse {
            faces: response.faces.into_iter().map(to_face_info).collect(),
        })
    }

    pub async fn delete_face(&self, req: &DeleteFaceRequest) -> PVResult<DeleteFaceResponse> {
        let mut client = self.identity_client().await?;

        let grpc_req = identity::DeleteFacesRequest {
            identity_id: req.fr_id.clone(),
            face_ids: vec![req.face_id.clone()],
        };

        let response = client
            .delete_faces(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(DeleteFaceResponse {
            rows_affected: response.rows_affected,
        })
    }

    pub async fn get_faces(&self, req: GetFacesRequest) -> PVResult<GetFacesResponse> {
        let mut client = self.identity_client().await?;

        let grpc_req = identity::GetFacesRequest {
            identity_id: req.fr_id,
            page_token: String::new(),
            page_size: DEFAULT_PAGE_SIZE as i32,
        };

        let response = client.get_faces(Request::new(grpc_req)).await?.into_inner();

        Ok(GetFacesResponse {
            faces: response.faces.into_iter().map(to_face_info).collect(),
            next_page_token: response.next_page_token,
            total_size: response.total_size,
        })
    }

    async fn identity_client(
        &self,
    ) -> PVResult<identity::identity_service_client::IdentityServiceClient<Channel>> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone()).map_err(|e| {
            PVApiError::with_code(500, &format!("invalid identity endpoint: {}", e))
        })?;
        let channel = endpoint.connect().await?;
        Ok(identity::identity_service_client::IdentityServiceClient::new(channel))
    }
}

fn normalize_endpoint(endpoint: String) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    if endpoint.contains("://") {
        endpoint
    } else {
        format!("http://{}", endpoint)
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

fn to_lookup_identities(response: identity::LookupResponse) -> LookupIdentities {
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
