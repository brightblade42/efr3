use bytes::Bytes;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::grpc_utils::normalize_endpoint;
use crate::proc_mapper::{
    health_status_label, to_liveness_process_image_request, to_process_image_request,
    to_process_image_response,
};
use crate::types::{HealthCheckResponse, ProcessImageResponse};

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

    pub async fn process_image(
        &self,
        image: Bytes,
        outputs: Option<Vec<String>>,
        find_most_prominent_face: bool,
    ) -> PVResult<ProcessImageResponse> {
        let mut client = self.processor_client().await?;
        let request = to_process_image_request(image, outputs, find_most_prominent_face)?;

        let response = client
            .process_full_image(Request::new(request))
            .await?
            .into_inner();

        Ok(to_process_image_response(response))
    }

    pub async fn process_image_liveness(&self, image: Bytes) -> PVResult<ProcessImageResponse> {
        let mut client = self.processor_client().await?;
        let request = to_liveness_process_image_request(image)?;

        let response = client
            .process_full_image(Request::new(request))
            .await?
            .into_inner();

        Ok(to_process_image_response(response))
    }

    pub async fn health_check(&self) -> PVResult<HealthCheckResponse> {
        let mut client = self.health_client().await?;
        let request = health::HealthCheckRequest {
            service: String::new(),
        };

        let response = client.check(Request::new(request)).await?.into_inner();
        Ok(HealthCheckResponse {
            status: health_status_label(response.status),
        })
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
