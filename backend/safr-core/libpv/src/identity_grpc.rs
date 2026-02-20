use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::grpc_utils::normalize_endpoint;

type PVResult<T> = Result<T, PVApiError>;

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

    pub async fn get_identities(
        &self,
        req: identity::GetIdentitiesRequest,
    ) -> PVResult<identity::GetIdentitiesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client.get_identities(Request::new(req)).await?.into_inner())
    }

    pub async fn create_identities(
        &self,
        req: identity::CreateIdentitiesRequest,
    ) -> PVResult<identity::CreateIdentitiesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client
            .create_identities(Request::new(req))
            .await?
            .into_inner())
    }

    pub async fn delete_identities(
        &self,
        req: identity::DeleteIdentitiesRequest,
    ) -> PVResult<identity::DeleteIdentitiesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client
            .delete_identities(Request::new(req))
            .await?
            .into_inner())
    }

    pub async fn lookup(&self, req: identity::LookupRequest) -> PVResult<identity::LookupResponse> {
        let mut client = self.identity_client().await?;
        Ok(client.lookup(Request::new(req)).await?.into_inner())
    }

    pub async fn add_faces(
        &self,
        req: identity::AddFacesRequest,
    ) -> PVResult<identity::AddFacesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client.add_faces(Request::new(req)).await?.into_inner())
    }

    pub async fn delete_faces(
        &self,
        req: identity::DeleteFacesRequest,
    ) -> PVResult<identity::DeleteFacesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client.delete_faces(Request::new(req)).await?.into_inner())
    }

    pub async fn get_faces(
        &self,
        req: identity::GetFacesRequest,
    ) -> PVResult<identity::GetFacesResponse> {
        let mut client = self.identity_client().await?;
        Ok(client.get_faces(Request::new(req)).await?.into_inner())
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
