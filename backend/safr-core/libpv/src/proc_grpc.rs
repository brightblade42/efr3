use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::grpc_utils::normalize_endpoint;

type PVResult<T> = Result<T, PVApiError>;

pub mod health {
    tonic::include_proto!("grpc.health.v1");
}

pub mod processor {
    tonic::include_proto!("processor.v7");
}

#[derive(Clone)]
pub struct PVProcGrpcApi {
    endpoint: String,
}

impl PVProcGrpcApi {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint: normalize_endpoint(endpoint),
        }
    }

    pub async fn process_full_image(
        &self,
        req: processor::ProcessFullImageRequest,
    ) -> PVResult<processor::ProcessFullImageResponse> {
        let mut client = self.processor_client().await?;
        Ok(client
            .process_full_image(Request::new(req))
            .await?
            .into_inner())
    }

    pub async fn health_check(&self) -> PVResult<health::HealthCheckResponse> {
        let mut client = self.health_client().await?;
        let request = health::HealthCheckRequest {
            service: String::new(),
        };
        Ok(client.check(Request::new(request)).await?.into_inner())
    }

    async fn processor_client(
        &self,
    ) -> PVResult<processor::processor_service_client::ProcessorServiceClient<Channel>> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone()).map_err(|e| {
            PVApiError::with_code(500, &format!("invalid processor endpoint: {}", e))
        })?;
        let channel = endpoint.connect().await?;
        Ok(processor::processor_service_client::ProcessorServiceClient::new(channel))
    }

    async fn health_client(&self) -> PVResult<health::health_client::HealthClient<Channel>> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone())
            .map_err(|e| PVApiError::with_code(500, &format!("invalid health endpoint: {}", e)))?;
        let channel = endpoint.connect().await?;
        Ok(health::health_client::HealthClient::new(channel))
    }
}
