use libfr::repo::{
    ExternalId, ImageRecord, ProfileRecord, RegistrationErrorRecord, SqlxFrRepository,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type RepoSetupResult = Result<(sqlx::PgPool, SqlxFrRepository), Box<dyn std::error::Error>>;

const IDENTITY_DB_URL_ENV: &str = "IDENTITY_DB_URL";

#[tokio::test]
#[ignore = "requires writable integration database"]
async fn upsert_get_delete_profile_roundtrip() -> TestResult {
    let (pool, repo) = connect_repo().await?;
    let external_id = ExternalId::new(format!("eyefr-rs-test-{}", unix_millis()))?;
    let profile = ProfileRecord {
        external_id: external_id.clone(),
        first_name: Some("Integration".to_string()),
        last_name: Some("Roundtrip".to_string()),
        middle_name: Some("DB".to_string()),
        image_url: Some("https://example.test/image.jpg".to_string()),
        raw_data: Some(json!({"type": "integration-test", "compId": 999})),
        fr_id: None,
    };

    repo.upsert_profile(&profile).await?;

    let fetched = repo
        .get_profile_by_external_id(&external_id)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "profile not found after upsert"))?;

    assert_eq!(fetched.external_id.as_str(), external_id.as_str());
    assert_eq!(fetched.first_name.as_deref(), Some("Integration"));
    assert_eq!(fetched.last_name.as_deref(), Some("Roundtrip"));

    let deleted = repo.delete_profile_by_external_id(&external_id).await?;
    assert_eq!(deleted, 1, "expected to delete exactly one profile row");

    let maybe_deleted = repo.get_profile_by_external_id(&external_id).await?;
    assert!(
        maybe_deleted.is_none(),
        "profile should not exist after delete"
    );

    pool.close().await;

    Ok(())
}

#[tokio::test]
#[ignore = "requires writable integration database"]
async fn search_profiles_by_last_name_roundtrip() -> TestResult {
    let (pool, repo) = connect_repo().await?;
    let marker = format!("RsSearch{}", unix_millis());
    let external_id = ExternalId::new(format!("eyefr-rs-test-search-{}", unix_millis()))?;

    let profile = ProfileRecord {
        external_id: external_id.clone(),
        first_name: Some("Search".to_string()),
        last_name: Some(marker.clone()),
        middle_name: None,
        image_url: None,
        raw_data: Some(json!({"type": "integration-search"})),
        fr_id: Some(format!("fr-{}", marker)),
    };

    repo.upsert_profile(&profile).await?;

    let hits = repo.search_profiles_by_last_name(&marker, 10).await?;
    assert!(
        hits.iter()
            .any(|item| item.external_id.as_str() == external_id.as_str()),
        "expected search results to include inserted profile"
    );

    let roster = repo.get_enrollment_roster(100).await?;
    assert!(
        !roster.is_empty(),
        "expected roster call to return at least one profile"
    );

    let metadata = repo.get_enrollment_metadata().await?;
    assert!(metadata.profiles_total >= 1);
    assert!(metadata.profiles_with_fr_id >= 1);
    assert!(metadata.images_total >= 0);
    assert!(metadata.registration_errors_total >= 0);
    assert!(metadata.enrollment_logs_total >= 0);

    let exact = repo
        .find_profile_by_name("Search", &marker, None)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "expected exact name match"))?;
    assert_eq!(exact.external_id.as_str(), external_id.as_str());

    let case_insensitive = repo
        .find_profile_by_name("search", &marker.to_lowercase(), None)
        .await?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "expected case-insensitive name match",
            )
        })?;
    assert_eq!(case_insensitive.external_id.as_str(), external_id.as_str());

    let deleted = repo.delete_profile_by_external_id(&external_id).await?;
    assert_eq!(deleted, 1, "expected to delete inserted search profile");

    pool.close().await;

    Ok(())
}

#[tokio::test]
#[ignore = "requires writable integration database"]
async fn upsert_get_delete_image_roundtrip() -> TestResult {
    let (pool, repo) = connect_repo().await?;
    let external_id = ExternalId::new(format!("eyefr-rs-test-img-{}", unix_millis()))?;
    let image = ImageRecord {
        external_id: external_id.clone(),
        data: vec![1, 2, 3, 4, 5],
        size: Some(5.0),
        url: Some("https://example.test/image.jpg".to_string()),
        quality: 0.91,
        acceptability: 0.88,
        raw_data: Some(json!({"kind": "integration-test-image"})),
    };

    repo.upsert_image(&image).await?;

    let fetched = repo
        .get_image_by_external_id(&external_id)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "image not found after upsert"))?;

    assert_eq!(fetched.external_id.as_str(), external_id.as_str());
    assert_eq!(fetched.data, image.data);
    assert_eq!(fetched.quality, image.quality);
    assert_eq!(fetched.acceptability, image.acceptability);

    let deleted = repo.delete_image_by_external_id(&external_id).await?;
    assert_eq!(deleted, 1, "expected to delete exactly one image row");

    let maybe_deleted = repo.get_image_by_external_id(&external_id).await?;
    assert!(
        maybe_deleted.is_none(),
        "image should not exist after delete"
    );

    pool.close().await;

    Ok(())
}

#[tokio::test]
#[ignore = "requires writable integration database"]
async fn insert_and_read_fr_logs_roundtrip() -> TestResult {
    let (pool, repo) = connect_repo().await?;
    let marker = format!("eyefr-rs-test-log-{}", unix_millis());
    let external_id = ExternalId::new(marker.clone())?;
    let reg_message = format!("registration issue {}", marker);
    let enrollment_code = format!("enroll-{}", marker);
    let enrollment_payload = json!({"test_marker": marker});

    let reg_error = RegistrationErrorRecord {
        external_id: Some(external_id.clone()),
        fr_id: Some("fr-test-id".to_string()),
        message: Some(reg_message.clone()),
    };

    repo.insert_registration_error(&reg_error).await?;

    let reg_rows = repo
        .get_registration_errors_by_external_id(&external_id, 10)
        .await?;
    assert!(
        reg_rows
            .iter()
            .any(|row| row.message.as_deref() == Some(reg_message.as_str())),
        "expected registration error entry with marker"
    );

    repo.append_enrollment_log(&enrollment_code, &enrollment_payload)
        .await?;

    let enroll_rows = repo
        .get_enrollment_logs_by_code(&enrollment_code, 10)
        .await?;
    assert!(
        enroll_rows
            .iter()
            .any(|row| row.payload == enrollment_payload),
        "expected enrollment log payload with marker"
    );

    let recent_rows = repo.get_enrollment_logs(25).await?;
    assert!(
        recent_rows.iter().any(|row| row.code == enrollment_code),
        "expected recent enrollment logs to contain inserted code"
    );

    let _ = sqlx::query(
        r#"
        delete from eyefr.registration_errors
        where ext_id = $1 and message = $2
        "#,
    )
    .bind(external_id.as_str())
    .bind(&reg_message)
    .execute(&pool)
    .await?;

    let _ = sqlx::query(
        r#"
        delete from logs.enrollment
        where code = $1 and payload = $2
        "#,
    )
    .bind(&enrollment_code)
    .bind(&enrollment_payload)
    .execute(&pool)
    .await?;

    pool.close().await;

    Ok(())
}

async fn connect_repo() -> RepoSetupResult {
    let db_url = env::var(IDENTITY_DB_URL_ENV)?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await?;
    let repo = SqlxFrRepository::new(pool.clone());
    Ok((pool, repo))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
