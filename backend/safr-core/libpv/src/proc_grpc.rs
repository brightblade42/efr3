use std::sync::Arc;

use tokio::sync::OnceCell;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::grpc_utils::normalize_endpoint;

type PVResult<T> = Result<T, PVApiError>;

//NOTE: keeps generated grpc code out of our project but gives us
//an easy way to use it.
pub mod health {
    tonic::include_proto!("grpc.health.v1");
}

pub mod processor {
    tonic::include_proto!("processor.v7");
}

#[derive(Clone)]
pub struct PVProcGrpcApi {
    endpoint: String,
    channel: Arc<OnceCell<Channel>>,
}

impl PVProcGrpcApi {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint: normalize_endpoint(endpoint), channel: Arc::new(OnceCell::new()) }
    }

    pub async fn process_full_image(
        &self,
        req: processor::ProcessFullImageRequest,
    ) -> PVResult<processor::ProcessFullImageResponse> {
        let mut client = self.processor_client().await?;
        Ok(client.process_full_image(Request::new(req)).await?.into_inner())
    }

    pub async fn health_check(&self) -> PVResult<health::HealthCheckResponse> {
        let mut client = self.health_client().await?;
        let request = health::HealthCheckRequest { service: String::new() };
        Ok(client.check(Request::new(request)).await?.into_inner())
    }

    async fn processor_client(
        &self,
    ) -> PVResult<processor::processor_service_client::ProcessorServiceClient<Channel>> {
        Ok(processor::processor_service_client::ProcessorServiceClient::new(
            self.channel().await?,
        ))
    }

    async fn health_client(&self) -> PVResult<health::health_client::HealthClient<Channel>> {
        Ok(health::health_client::HealthClient::new(self.channel().await?))
    }

    async fn channel(&self) -> PVResult<Channel> {
        let endpoint = self.endpoint.clone();
        let channel = self
            .channel
            .get_or_try_init(|| async move {
                let endpoint = Endpoint::from_shared(endpoint).map_err(|e| {
                    PVApiError::with_code(500, &format!("invalid processor endpoint: {}", e))
                })?;
                Ok::<Channel, PVApiError>(endpoint.connect_lazy())
            })
            .await?;

        Ok(channel.clone())
    }
}
