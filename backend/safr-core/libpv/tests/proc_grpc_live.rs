use bytes::Bytes;
use libpv::proc_grpc::PVProcGrpcApi;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ENDPOINT_ENV: &str = "PV_PROC_GRPC_SMOKE_ENDPOINT";
const IMAGE_DIR_ENV: &str = "PV_PROC_GRPC_SMOKE_IMAGE_DIR";

#[tokio::test]
#[ignore = "requires live Paravision processor endpoint"]
async fn live_proc_grpc_health_check() -> TestResult {
    let endpoint = required_env(ENDPOINT_ENV)?;
    let api = PVProcGrpcApi::new(endpoint);

    let health = api.health_check().await?;
    println!("health status: {}", health.status);

    assert!(
        !health.status.is_empty(),
        "health status should not be empty"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Paravision processor endpoint and local test images"]
async fn live_proc_grpc_process_images() -> TestResult {
    let endpoint = required_env(ENDPOINT_ENV)?;
    let image_dir = PathBuf::from(required_env(IMAGE_DIR_ENV)?);
    let image_paths = collect_jpeg_images(&image_dir)?;

    assert!(
        !image_paths.is_empty(),
        "no jpg/jpeg files found in {}",
        image_dir.display()
    );

    let api = PVProcGrpcApi::new(endpoint);
    let mut processed = 0usize;
    let mut skipped_invalid_images = 0usize;

    for image_path in image_paths {
        let bytes = fs::read(&image_path)?;
        let response = match api.process_image(Bytes::from(bytes), None, true).await {
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
                skipped_invalid_images += 1;
                continue;
            }
            Err(err) => return Err(err.into()),
        };

        let face_count = response.faces.as_ref().map_or(0usize, Vec::len);
        if let Some(idx) = response.most_prominent_face_idx {
            if face_count > 0 {
                assert!(
                    (idx as usize) < face_count,
                    "most prominent face index {} out of bounds {} for {}",
                    idx,
                    face_count,
                    image_path.display()
                );
            }
        }

        println!(
            "processed {} -> faces={}, most_prominent_face_idx={:?}",
            image_path.display(),
            face_count,
            response.most_prominent_face_idx
        );
        processed += 1;
    }

    println!(
        "processed {} image(s), skipped {} invalid image file(s)",
        processed, skipped_invalid_images
    );
    assert!(processed > 0, "expected at least one processed image");
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Paravision processor endpoint and local test images"]
async fn live_proc_grpc_liveness_check() -> TestResult {
    let endpoint = required_env(ENDPOINT_ENV)?;
    let image_dir = PathBuf::from(required_env(IMAGE_DIR_ENV)?);
    let image_paths = collect_jpeg_images(&image_dir)?;

    assert!(
        !image_paths.is_empty(),
        "no jpg/jpeg files found in {}",
        image_dir.display()
    );

    let api = PVProcGrpcApi::new(endpoint);
    let mut successful_checks = 0usize;

    for image_path in image_paths {
        let bytes = fs::read(&image_path)?;
        let response = match api.process_image_liveness(Bytes::from(bytes)).await {
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

        let faces = response.faces.unwrap_or_default();
        if faces.is_empty() {
            println!("skipping {} -> no faces detected", image_path.display());
            continue;
        }

        let idx = response
            .most_prominent_face_idx
            .filter(|idx| *idx >= 0)
            .map(|idx| idx as usize)
            .unwrap_or(0);
        let face = faces.get(idx).or_else(|| faces.first()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "expected at least one face in liveness response",
            )
        })?;

        let liveness = face.liveness.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("missing liveness payload for {}", image_path.display()),
            )
        })?;

        let validness = face.liveness_validness.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "missing liveness validness payload for {}",
                    image_path.display()
                ),
            )
        })?;

        println!(
            "liveness {} -> prob={}, is_valid={}, feedback={:?}",
            image_path.display(),
            liveness.liveness_probability,
            validness.is_valid,
            validness.feedback
        );

        successful_checks += 1;
    }

    assert!(
        successful_checks > 0,
        "expected at least one successful liveness check"
    );
    Ok(())
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
