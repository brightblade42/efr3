use bytes::Bytes;
use libpv::identity_grpc::PVIdentityGrpcApi;
use libpv::proc_grpc::{
    health::{
        health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    },
    PVProcGrpcApi,
};
use libpv::types::{
    AddFaceRequest, CreateIdentitiesRequest, DeleteIdentitiesRequest, Embedding, GetFacesRequest,
};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::Endpoint;
use tonic::Request;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PROC_ENDPOINT_ENV: &str = "PV_PROC_GRPC_SMOKE_ENDPOINT";
const IDENT_ENDPOINT_ENV: &str = "PV_IDENT_GRPC_SMOKE_ENDPOINT";
const IMAGE_DIR_ENV: &str = "PV_PROC_GRPC_SMOKE_IMAGE_DIR";
const SYNTHETIC_DIMENSION: usize = 128;

#[derive(Clone)]
struct FaceTemplate {
    path: PathBuf,
    embedding: Vec<f32>,
    quality: f32,
}

#[tokio::test]
#[ignore = "requires live Paravision identity endpoint"]
async fn live_identity_grpc_health_check() -> TestResult {
    let endpoint = normalize_endpoint(&required_env(IDENT_ENDPOINT_ENV)?);
    let channel = Endpoint::from_shared(endpoint)?.connect().await?;
    let mut client = HealthClient::new(channel);

    let response = client
        .check(Request::new(HealthCheckRequest {
            service: String::new(),
        }))
        .await?
        .into_inner();

    let status = ServingStatus::try_from(response.status).unwrap_or(ServingStatus::Unknown);
    println!("identity health status: {:?}", status);

    if status == ServingStatus::Unknown {
        return Err(
            io::Error::new(io::ErrorKind::Other, "identity health returned UNKNOWN").into(),
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Paravision proc/identity endpoints and local images"]
async fn live_identity_grpc_create_add_face_lookup_delete_roundtrip() -> TestResult {
    let ident_endpoint = required_env(IDENT_ENDPOINT_ENV)?;
    let ident_api = PVIdentityGrpcApi::new(ident_endpoint);
    let templates = templates_from_proc_or_synthetic().await;

    let first = templates.first().cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "expected at least one extracted face template",
        )
    })?;
    let second = templates.get(1).cloned().unwrap_or_else(|| first.clone());

    let external_id = format!("grpc-smoke-{}", unix_millis());
    let create_req = CreateIdentitiesRequest {
        embeddings: vec![Embedding {
            embedding: first.embedding.clone(),
        }],
        threshold: 0.0,
        qualities: vec![first.quality],
        group_ids: None,
        external_ids: Some(vec![external_id.clone()]),
    };

    let created = ident_api.create_identities(create_req).await?;
    let created_id = created
        .identities
        .first()
        .map(|identity| identity.id.clone())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "identity create returned no identities",
            )
        })?;

    println!(
        "created identity {} from {} (external_id={})",
        created_id,
        first.path.display(),
        external_id
    );

    let mut issues: Vec<String> = Vec::new();

    let add_req = AddFaceRequest {
        identity_id: created_id.clone(),
        embeddings: vec![Embedding {
            embedding: second.embedding.clone(),
        }],
        threshold: 0.0,
        qualities: vec![second.quality],
    };

    match ident_api.add_face(add_req).await {
        Ok(add_resp) => {
            println!(
                "added {} face(s) to {} using {}",
                add_resp.faces.len(),
                created_id,
                second.path.display()
            );
            if add_resp.faces.is_empty() {
                issues.push("add_face returned zero faces".to_string());
            } else if add_resp.faces[0].identity_id != created_id {
                issues.push(format!(
                    "add_face returned mismatched identity_id: expected {}, got {}",
                    created_id, add_resp.faces[0].identity_id
                ));
            }
        }
        Err(err) => issues.push(format!("add_face failed: {}", err)),
    }

    match ident_api
        .get_faces(GetFacesRequest {
            fr_id: created_id.clone(),
        })
        .await
    {
        Ok(faces_resp) => {
            println!(
                "get_faces for {} -> page_len={}, total_size={}",
                created_id,
                faces_resp.faces.len(),
                faces_resp.total_size
            );
            if faces_resp.faces.is_empty() && faces_resp.total_size <= 0 {
                issues.push("get_faces returned no face records".to_string());
            } else if let Some(face) = faces_resp.faces.first() {
                if face.identity_id != created_id {
                    issues.push(format!(
                        "get_faces returned mismatched identity_id: expected {}, got {}",
                        created_id, face.identity_id
                    ));
                }
            }
        }
        Err(err) => issues.push(format!("get_faces failed: {}", err)),
    }

    match ident_api
        .lookup_single(Embedding {
            embedding: first.embedding.clone(),
        })
        .await
    {
        Ok(lookup_resp) => {
            let match_count = lookup_resp
                .lookup_identities
                .first()
                .map_or(0usize, |item| item.matches.len());
            println!(
                "lookup_single from created embedding -> match_count={} (created_id={})",
                match_count, created_id
            );
            if match_count == 0 {
                issues.push("lookup_single returned no matches".to_string());
            }
        }
        Err(err) => issues.push(format!("lookup_single failed: {}", err)),
    }

    let delete_results = ident_api
        .delete_identities(Some(DeleteIdentitiesRequest {
            ids: vec![created_id.clone()],
            external_ids: None,
        }))
        .await?;
    let deleted = delete_results
        .into_iter()
        .any(|result| matches!(result, Ok(id) if id == created_id));

    println!("cleanup delete identity {} -> {}", created_id, deleted);
    if !deleted {
        issues.push(format!(
            "cleanup failed: identity {} was not deleted",
            created_id
        ));
    }

    if !issues.is_empty() {
        return Err(io::Error::new(io::ErrorKind::Other, issues.join("; ")).into());
    }

    Ok(())
}

async fn collect_templates(
    proc_api: &PVProcGrpcApi,
    image_paths: &[PathBuf],
) -> Result<Vec<FaceTemplate>, Box<dyn std::error::Error>> {
    let mut templates: Vec<FaceTemplate> = Vec::new();

    for image_path in image_paths {
        let bytes = fs::read(image_path)?;
        let response = match proc_api.process_image(Bytes::from(bytes), None, true).await {
            Ok(response) => response,
            Err(err)
                if err.code == 400
                    && err
                        .message
                        .to_ascii_lowercase()
                        .contains("not a valid image") =>
            {
                println!(
                    "skipping {} -> invalid image payload reported by processor",
                    image_path.display()
                );
                continue;
            }
            Err(err) => return Err(err.into()),
        };

        let Some((embedding, quality)) = extract_face_template(response) else {
            println!(
                "skipping {} -> no face embedding in process response",
                image_path.display()
            );
            continue;
        };

        println!(
            "template from {} -> embedding_len={}, quality={}",
            image_path.display(),
            embedding.len(),
            quality
        );

        templates.push(FaceTemplate {
            path: image_path.clone(),
            embedding,
            quality,
        });
    }

    Ok(templates)
}

async fn templates_from_proc_or_synthetic() -> Vec<FaceTemplate> {
    let proc_endpoint = env::var(PROC_ENDPOINT_ENV).ok();
    let image_dir = env::var(IMAGE_DIR_ENV).ok().map(PathBuf::from);

    if let (Some(proc_endpoint), Some(image_dir)) = (proc_endpoint, image_dir) {
        match collect_jpeg_images(&image_dir) {
            Ok(image_paths) => {
                let proc_api = PVProcGrpcApi::new(proc_endpoint);
                match collect_templates(&proc_api, &image_paths).await {
                    Ok(templates) if !templates.is_empty() => {
                        println!(
                            "using {} template(s) extracted from proc/image inputs",
                            templates.len()
                        );
                        return templates;
                    }
                    Ok(_) => {
                        println!(
                            "no templates extracted from {}; falling back to synthetic embeddings",
                            image_dir.display()
                        );
                    }
                    Err(err) => {
                        println!(
                            "could not collect proc templates ({}); falling back to synthetic embeddings",
                            err
                        );
                    }
                }
            }
            Err(err) => {
                println!(
                    "could not enumerate image dir {} ({}); falling back to synthetic embeddings",
                    image_dir.display(),
                    err
                );
            }
        }
    } else {
        println!(
            "missing {} or {}; falling back to synthetic embeddings",
            PROC_ENDPOINT_ENV, IMAGE_DIR_ENV
        );
    }

    vec![synthetic_template(1.0), synthetic_template(2.0)]
}

fn extract_face_template(response: libpv::types::ProcessImageResponse) -> Option<(Vec<f32>, f32)> {
    let faces = response.faces?;
    if faces.is_empty() {
        return None;
    }

    let candidate_index = response
        .most_prominent_face_idx
        .filter(|idx| *idx >= 0)
        .map(|idx| idx as usize)
        .unwrap_or(0);

    let face = faces.get(candidate_index).or_else(|| faces.first())?;
    let embedding = face.embedding.clone()?;
    let quality = face.quality.unwrap_or(0.0);
    Some((embedding, quality))
}

fn required_env(name: &str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("required env var {} is not set", name),
        )
    })
}

fn collect_jpeg_images(dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut images = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(ext) = path.extension().and_then(|item| item.to_str()) else {
            continue;
        };

        if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
            images.push(path);
        }
    }

    images.sort();
    Ok(images)
}

fn normalize_endpoint(endpoint: &str) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/');
    if endpoint.contains("://") {
        endpoint.to_string()
    } else {
        format!("http://{}", endpoint)
    }
}

fn synthetic_template(seed: f64) -> FaceTemplate {
    let embedding = (0..SYNTHETIC_DIMENSION)
        .map(|index| (((index as f64) + seed) / (SYNTHETIC_DIMENSION as f64)) as f32)
        .collect();

    FaceTemplate {
        path: PathBuf::from(format!("synthetic-{}", seed)),
        embedding,
        quality: 0.9,
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
