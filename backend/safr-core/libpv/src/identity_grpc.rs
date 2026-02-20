use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::grpc_utils::normalize_endpoint;
use crate::identity_mapper::{
    to_add_face_request, to_add_face_response, to_create_identities_request,
    to_create_identities_response, to_delete_face_response, to_delete_faces_request,
    to_delete_identities_request, to_get_faces_request, to_get_faces_response,
    to_get_identities_request, to_identities, to_lookup_identities, to_lookup_request,
};
use crate::types::{
    AddFaceInput, AddFaceResponse, CreateIdentitiesInput, CreateIdentitiesResponse,
    DeleteFaceInput, DeleteFaceResponse, DeleteIdentitiesInput, Embedding, Face, GetFacesInput,
    GetFacesResponse, GetIdentitiesInput, Identities, LookupIdentities, LookupInput,
    LookupResponse,
};

type PVResult<T> = Result<T, PVApiError>;
type PVResultMany<T> = PVResult<Vec<PVResult<T>>>;

const DEFAULT_PAGE_SIZE: u32 = 100;

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

    pub async fn get_identities(&self, req: Option<GetIdentitiesInput>) -> PVResult<Identities> {
        let mut client = self.identity_client().await?;
        let req = req.unwrap_or(GetIdentitiesInput {
            page_size: DEFAULT_PAGE_SIZE,
            page_token: Some(String::new()),
            group_ids: None,
        });

        let grpc_req = to_get_identities_request(req);

        let response = client
            .get_identities(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(to_identities(response))
    }

    pub async fn create_identities(
        &self,
        req: CreateIdentitiesInput,
    ) -> PVResult<CreateIdentitiesResponse> {
        let mut client = self.identity_client().await?;
        let grpc_req = to_create_identities_request(req);

        let response = client
            .create_identities(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(to_create_identities_response(response))
    }

    pub async fn delete_identities(
        &self,
        delete_req: Option<DeleteIdentitiesInput>,
    ) -> PVResultMany<String> {
        let delete_targets: Vec<(Option<String>, Option<String>, String)> = match delete_req {
            None => {
                let req = Some(GetIdentitiesInput {
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
            let grpc_req = to_delete_identities_request(id, external_id);

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
        let req = to_lookup_request(vec![embedding], 1);

        let response = client.lookup(Request::new(req)).await?.into_inner();
        Ok(to_lookup_identities(response))
    }

    pub async fn lookup<I: Into<LookupInput>>(&self, req: I) -> PVResultMany<LookupResponse> {
        let req: LookupInput = req.into();
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

            let lookup_req = to_lookup_request(vec![Embedding { embedding }], limit);

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

    pub async fn add_face(&self, req: AddFaceInput) -> PVResult<AddFaceResponse> {
        let mut client = self.identity_client().await?;
        let grpc_req = to_add_face_request(req);

        let response = client.add_faces(Request::new(grpc_req)).await?.into_inner();
        Ok(to_add_face_response(response))
    }

    pub async fn delete_face(&self, req: &DeleteFaceInput) -> PVResult<DeleteFaceResponse> {
        let mut client = self.identity_client().await?;
        let grpc_req = to_delete_faces_request(req);

        let response = client
            .delete_faces(Request::new(grpc_req))
            .await?
            .into_inner();

        Ok(to_delete_face_response(response))
    }

    pub async fn get_faces(&self, req: GetFacesInput) -> PVResult<GetFacesResponse> {
        let mut client = self.identity_client().await?;
        let grpc_req = to_get_faces_request(req, DEFAULT_PAGE_SIZE);

        let response = client.get_faces(Request::new(grpc_req)).await?.into_inner();

        Ok(to_get_faces_response(response))
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
