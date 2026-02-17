use bytes::Bytes;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;

use crate::errors::PVApiError;
use crate::types::{
    BoundingBox, Face, HealthCheckResponse, Landmarks, Liveness, Point, ProcessImageResponse,
    Validness,
};

type PVResult<T> = Result<T, PVApiError>;

pub mod health {
    tonic::include_proto!("grpc.health.v1");
}

pub mod processor {
    tonic::include_proto!("processor.v7");
}

const DEFAULT_OUTPUTS: [&str; 4] = ["BOUNDING_BOX", "EMBEDDING", "QUALITY", "MASK"];
const LIVENESS_OUTPUTS: [&str; 4] = ["BOUNDING_BOX", "QUALITY", "LIVENESS", "LIVENESS_VALIDNESS"];

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

        let request = processor::ProcessFullImageRequest {
            image: image.to_vec(),
            outputs: map_outputs(outputs)?,
            find_most_prominent_face,
            scoring_mode: processor::ScoringMode::Auto as i32,
            image_source: processor::ImageSource::Unknown as i32,
            liveness_validness_parameters: None,
            ages_v2_validness_parameters: None,
            deepfake_validness_parameters: None,
        };

        let response = client
            .process_full_image(Request::new(request))
            .await?
            .into_inner();

        Ok(to_process_image_response(response))
    }

    pub async fn process_image_liveness(&self, image: Bytes) -> PVResult<ProcessImageResponse> {
        let mut client = self.processor_client().await?;

        let request = processor::ProcessFullImageRequest {
            image: image.to_vec(),
            outputs: map_outputs(Some(
                LIVENESS_OUTPUTS
                    .iter()
                    .map(|item| (*item).to_string())
                    .collect(),
            ))?,
            find_most_prominent_face: true,
            scoring_mode: processor::ScoringMode::Auto as i32,
            image_source: processor::ImageSource::Webcam as i32,
            liveness_validness_parameters: Some(default_liveness_validness_parameters()),
            ages_v2_validness_parameters: None,
            deepfake_validness_parameters: None,
        };

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

fn normalize_endpoint(endpoint: String) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    if endpoint.contains("://") {
        endpoint
    } else {
        format!("http://{}", endpoint)
    }
}

fn map_outputs(outputs: Option<Vec<String>>) -> PVResult<Vec<i32>> {
    let outputs = outputs.unwrap_or_else(|| {
        DEFAULT_OUTPUTS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    });

    outputs
        .into_iter()
        .map(|output| map_output(&output))
        .collect()
}

fn map_output(output: &str) -> PVResult<i32> {
    use processor::process_full_image_request::Options;

    let normalized = output.trim().to_ascii_uppercase();
    let mapped = match normalized.as_str() {
        "BOUNDING_BOX" => Options::BoundingBox,
        "LANDMARKS" => Options::Landmarks,
        "ALIGNED_FACE_IMAGE" => Options::AlignedFaceImage,
        "ATTRIBUTES_IMAGES" => Options::AttributesImages,
        "DEEPFAKE_IMAGES" => Options::DeepfakeImages,
        "LIVENESS_IMAGE" => Options::LivenessImage,
        "AGES_V2_IMAGE" => Options::AgesV2Image,
        "QUALITY" => Options::Quality,
        "EMBEDDING" => Options::Embedding,
        "AGES" => Options::Ages,
        "AGES_V2_VALIDNESS" => Options::AgesV2Validness,
        "AGES_V2" => Options::AgesV2,
        "GENDERS" => Options::Genders,
        "MASK" => Options::Mask,
        "HEADWEAR" => Options::Headwear,
        "GLASSES" => Options::Glasses,
        "EYES" => Options::Eyes,
        "SMILE" => Options::Smile,
        "LIVENESS_VALIDNESS" => Options::LivenessValidness,
        "LIVENESS" => Options::Liveness,
        "DEEPFAKE" => Options::Deepfake,
        "DEEPFAKE_VALIDNESS" => Options::DeepfakeValidness,
        "ADVANCED_DATA" => Options::AdvancedData,
        _ => {
            return Err(PVApiError::with_code(
                400,
                &format!("unsupported process_full_image output: {}", output),
            ))
        }
    };

    Ok(mapped as i32)
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

fn to_process_image_response(
    response: processor::ProcessFullImageResponse,
) -> ProcessImageResponse {
    let faces = if response.faces.is_empty() {
        None
    } else {
        Some(response.faces.into_iter().map(to_face).collect())
    };

    ProcessImageResponse {
        faces,
        most_prominent_face_idx: Some(response.most_prominent_face_idx),
    }
}

fn to_face(face: processor::Face) -> Face {
    let embedding = if face.embedding.is_empty() {
        None
    } else {
        Some(face.embedding.into_iter().map(|item| item as f64).collect())
    };

    Face {
        bounding_box: face.bounding_box.map(to_bounding_box),
        landmarks: face.landmarks.map(to_landmarks),
        embedding,
        ages: None,
        genders: None,
        aligned_face_image: None,
        acceptability: Some(face.acceptability),
        quality: Some(face.quality),
        mask: Some(face.mask),
        liveness_validness: face.liveness_validness.map(to_validness),
        liveness: face.liveness.map(to_liveness),
    }
}

fn to_validness(validness: processor::Validness) -> Validness {
    use processor::validness::Feedback;

    Validness {
        is_valid: validness.is_valid,
        feedback: validness
            .feedback
            .into_iter()
            .map(|item| {
                Feedback::try_from(item)
                    .unwrap_or(Feedback::Unknown)
                    .as_str_name()
                    .to_string()
            })
            .collect(),
    }
}

fn to_liveness(liveness: processor::Liveness) -> Liveness {
    Liveness {
        liveness_probability: liveness.liveness_probability,
    }
}

fn to_bounding_box(bounding_box: processor::BoundingBox) -> BoundingBox {
    BoundingBox {
        origin: to_point(bounding_box.origin),
        width: bounding_box.width,
        height: bounding_box.height,
    }
}

fn to_landmarks(landmarks: processor::Landmarks) -> Landmarks {
    Landmarks {
        eye_left: to_point(landmarks.left_eye),
        eye_right: to_point(landmarks.right_eye),
        nose: to_point(landmarks.nose),
        mouth_left: to_point(landmarks.left_mouth),
        mouth_right: to_point(landmarks.right_mouth),
    }
}

fn to_point(point: Option<processor::Point>) -> Point {
    let point = point.unwrap_or_default();
    Point {
        x: point.x,
        y: point.y,
    }
}

fn health_status_label(status: i32) -> String {
    use health::health_check_response::ServingStatus;

    match ServingStatus::try_from(status).unwrap_or(ServingStatus::Unknown) {
        ServingStatus::Serving => "SERVING".to_string(),
        ServingStatus::NotServing => "NOT_SERVING".to_string(),
        ServingStatus::Unknown => "UNKNOWN".to_string(),
    }
}
