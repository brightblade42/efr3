#[macro_use]
mod macros;

mod config;
mod errors;
mod extractors;
mod handlers;

use crate::handlers::*;
use axum::http::{Method, StatusCode};
use dotenvy::dotenv;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::AppConfig;
use crate::errors::AppError;

use libfr::dispatch::{AssetDispatcher, FRDispatcher};
use libfr::repo::SqlxFrRepository;
use libfr::service::FRService;
use libfr::tpass::{api::TPassClient, config::TPassConf};

type WResult<T> = Result<T, AppError>;

// Backend/remote are selected once at startup via env defaults.
#[derive(Clone)]
struct AppState {
    fr_service: Arc<FRService>,
    fr_repo: Arc<SqlxFrRepository>,
    tpass_client: Arc<TPassClient>,
    config: AppConfig,
}

//V1 backport
fn api_v1_routes() -> Router<AppState> {
    Router::new()
        //         //NOTE: DEPRECATED, cam app uses
        .route("/recognize-faces-b64", post(attendance_handlers::mark_attendance_v1))
        .route("/recognize-faces", post(recognition_handlers::recognize))
        .route("/recognize", post(recognition_handlers::recognize_v1))
        .route("/enrollment/create", post(enrollment_handlers::create_enrollment_v1))
        .route("/enrollment/delete", post(enrollment_handlers::delete_enrollment_v1))
        .route("/enrollment/add-face", post(enrollment_handlers::add_face_v1))
        .route("/enrollment/delete-face", post(enrollment_handlers::delete_face_v1))
        .route("/get-identity", post(enrollment_handlers::get_faces_v1))
        .route("/create-profile", post(profile_handlers::create_profile))
        .route("/edit-profile", post(profile_handlers::edit_profile))
        .route("/send-alert", post(tpass_handlers::send_fr_alert))
}

fn api_v2_routes() -> Router<AppState> {
    Router::new()
        //enroll unenroll re-enroll search
        .route("/enrollment/create", post(enrollment_handlers::create_enrollment))
        .route("/enrollment/search", post(enrollment_handlers::search_enrollment))
        .route("/enrollment/delete", post(enrollment_handlers::delete_enrollment))
        .route("/enrollment/add-face", post(enrollment_handlers::add_face))
        .route("/enrollment/delete-faces", post(enrollment_handlers::delete_faces))
        .route("/enrollment/get-faces", post(enrollment_handlers::get_faces))
        //TODO: test profile (camera based) PROFILE interacts with REMOTE. essentially pass through to remote
        .route("/create-profile", post(profile_handlers::create_profile))
        .route("/edit-profile", post(profile_handlers::edit_profile))
        //TODO: test send alert with cam app
        .route("/send-alert", post(tpass_handlers::send_fr_alert))
        .route("/mark-attendance", post(attendance_handlers::mark_attendance))
        //NOTE: deprecated in favor of liveness-check, clearer name
        //TODO: delete validate-image after liveness demo is complete
        .route("/validate-image", post(recognition_handlers::liveness_check))
        //NOTE: liveness-check, does liveness and includes quality
        .route("/liveness-check", post(recognition_handlers::liveness_check))
        //just the quality. validate is a more verbose version
        .route("/quality-check", post(recognition_handlers::quality_check))
        .route("/detect-faces", post(recognition_handlers::detect_faces)) //detect, bbox.
        .route("/recognize", post(recognition_handlers::recognize))
        //a combo on recognition and notifying remote of building entrance / exit.
        //NOTE: this is a very dangerous function. maybe we block it.
        .route("/enrollment/reset", post(enrollment_handlers::reset_enrollments))
        //TODO: will need a paging strategy
        .route("/enrollment/errlog", post(enrollment_handlers::get_enrollment_errlog))
        //some summary info, counts of things.
        .route("/enrollment/metadata", get(enrollment_handlers::get_enrollment_metadata))
        //gets all the enrollments 1000 max atm
        .route("/enrollment/roster", get(enrollment_handlers::get_roster))
}

//NOTE: if TPASS is not the remote, these won't do shit.
fn tpass_routes() -> Router<AppState> {
    Router::new()
        .route("/get-companies", get(tpass_handlers::get_tpass_companies))
        .route("/get-client-types", get(tpass_handlers::get_tpass_client_types))
        .route("/get-status-types", get(tpass_handlers::get_tpass_status_types))
        //TODO: is this  something we use in production or was this just for testing?
        //might be better to elimate for security reasons. A tpass passthrough function is probably
        // not the best idea.
        .route("/search", post(tpass_handlers::search_tpass))
        .fallback(fallback)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()) //do I even need this? I may if I want to reduce tracing output as an optimization in prod
        .init();
    dotenv().ok();

    info!(target: "startup", "starting the web server!");
    info!(target: "startup", "hi ho");

    let config = match AppConfig::from_env() {
        Ok(conf) => conf,
        Err(e) => {
            error!(target: "startup", "{}", e);
            return;
        }
    };

    let db_conn = format!(
        "postgresql://{}:{}@{}:{}/{}?sslmode={}",
        &config.db_user,
        &config.db_pwd,
        &config.db_addr,
        &config.db_port,
        &config.db_name,
        &config.db_ssl_mode,
    );

    let db_pool = match PgPoolOptions::new()
        .min_connections(5)
        .max_connections(config.db_max_connections)
        .connect(&db_conn)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            error!("failed to initialize database pool: {}", e);
            return;
        }
    };

    let tp_conf = TPassConf::new(
        config.remote_url.as_str(),
        config.remote_user.as_str(),
        config.remote_pwd.as_str(),
    );
    //Arc'em up!
    let tpass_client = Arc::new(TPassClient::new(tp_conf));
    let fr_repo = Arc::new(SqlxFrRepository::new(db_pool.clone()));

    //NOTE: not sure about this RemoteRuntime business
    let remote = match AssetDispatcher::new(config.remote.as_str(), tpass_client.clone()) {
        Ok(remote) => remote,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    let remote = Arc::new(remote);

    //our fr backend
    let fr_engine = match FRDispatcher::new(
        config.engine.as_str(),
        format!("{}:{}", config.proc_addr, config.proc_port),
        format!("{}:{}", config.ident_addr, config.ident_port),
        db_pool.clone(),
    ) {
        Ok(fr_engine) => fr_engine,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    info!(target: "startup",
        "startup env FR_BACKEND={} FR_REMOTE={}",
        config.engine.as_str(),
        config.remote.as_str(),
    );

    let fr_service = Arc::new(FRService::new(Arc::new(fr_engine), remote, fr_repo.clone()));

    let app_state = AppState {
        fr_service,
        fr_repo,
        tpass_client,
        config: config.clone(), //some tpass specific calls
    };

    let app =
        Router::new()
            .nest("/fr", api_v1_routes())
            .nest("/fr/v2", api_v2_routes())
            .nest("/tpass", tpass_routes())
            //NOTE: i think we moved site serving out of here and up to the rev proxy
            .nest_service("/_app", ServeDir::new("./app/_app"))
            .layer(ServiceBuilder::new().layer(
                CorsLayer::new().allow_methods([Method::GET, Method::POST]).allow_origin(Any),
            ))
            .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!(target: "startup", "listening on {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!(target: "startup", "failed to bind listener on {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!(target: "startup","server error: {}", e);
    }
}

async fn fallback() -> (StatusCode, &'static str) {
    (
        StatusCode::NOT_FOUND,
        "I don't know what you think you're looking for but this ain't it. 404 bruh",
    )
}

//TODO: config add set up a folder for test images.
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use std::time::Duration;
    use tower::ServiceExt;

    async fn build_test_state() -> AppState {
        // Load config (assumes your .env or system env has the TPass test URL)
        let config = AppConfig::from_env().expect("must have an app config"); //_or_else(|_| AppConfig::default());

        // REAL DATABASE: Local test Postgres container
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(2))
            .connect("postgresql://admin:admin@127.0.0.1:5433/identity?sslmode=disable")
            .await
            .expect("Failed to connect to real test database on port 5433");

        // REAL TPASS: Pointing to the internal team's test server
        // (You can hardcode this here if it isn't in your AppConfig yet)
        let tpass_client = Arc::new(TPassClient::new(TPassConf {
            url: config.remote_url.clone(), // Or e.g., "https://tpass-test.internal"
            user: config.remote_user.clone(),
            pwd: config.remote_pwd.clone(),
        }));

        let remote = Arc::new(
            AssetDispatcher::new("tpass", tpass_client.clone())
                .expect("remote runtime should initialize"),
        );

        // REAL FR ENGINE: Local test Paravision container
        let fr_engine = FRDispatcher::new(
            "paravision",
            "127.0.0.1:50051".to_string(),
            "127.0.0.1:50052".to_string(),
            db_pool.clone(),
        )
        .expect("Failed to initialize real PV engine.");

        let fr_repo = Arc::new(SqlxFrRepository::new(db_pool));
        let fr_service = Arc::new(FRService::new(Arc::new(fr_engine), remote, fr_repo.clone()));

        AppState { fr_service, fr_repo, tpass_client, config }
    }

    async fn test_app() -> Router {
        Router::new()
            .nest("/fr", api_v1_routes())
            .nest("/fr/v2", api_v2_routes())
            .with_state(build_test_state().await)
    }

    // --- Helpers ---

    fn multipart_image_request(uri: &str) -> Request<Body> {
        let boundary = "X-BOUNDARY";

        // TODO: Replace "abc" with real JPEG bytes using `include_bytes!("test_face.jpg")`
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"face.jpg\"\r\nContent-Type: image/jpeg\r\n\r\nabc\r\n--{b}--\r\n",
            b = boundary
        );

        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", format!("multipart/form-data; boundary={}", boundary))
            .body(Body::from(body))
            .expect("multipart request")
    }

    fn multipart_enrollment_request(
        uri: &str,
        include_image: bool,
        include_details: bool,
        include_ext_id: bool,
    ) -> Request<Body> {
        let boundary = "X-BOUNDARY";
        let mut body = String::new();

        if include_image {
            // TODO: Replace "abc" with real JPEG bytes
            body.push_str(&format!(
                "--{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"face.jpg\"\r\nContent-Type: image/jpeg\r\n\r\nabc\r\n",
                b = boundary
            ));
        }

        if include_details {
            let details = if include_ext_id {
                r#"{"kind":"Min","first_name":"Test","last_name":"User","ext_id":"123"}"#
            } else {
                r#"{"kind":"Min","first_name":"Test","last_name":"User"}"#
            };

            body.push_str(&format!(
                "--{b}\r\nContent-Disposition: form-data; name=\"details\"\r\n\r\n{details}\r\n",
                b = boundary
            ));
        }

        body.push_str(&format!("--{}--\r\n", boundary));

        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", format!("multipart/form-data; boundary={}", boundary))
            .body(Body::from(body))
            .expect("multipart request")
    }

    async fn response_json(resp: axum::response::Response) -> Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.expect("response body bytes");
        serde_json::from_slice(&bytes).expect("json response")
    }

    // --- The Tests ---

    #[tokio::test]
    async fn add_face_requires_fr_id_query_param() {
        let app = test_app().await;
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/add-face")
            .body(Body::empty())
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_face_requires_face_id_field() {
        let app = test_app().await;
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/delete-face")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"fr_id":"abc"}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn get_identity_requires_fr_id_field() {
        let app = test_app().await;
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/get-identity")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn send_alert_requires_required_payload_fields() {
        let app = test_app().await;
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/send-alert")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn send_alert_happy_path_hits_real_tpass() {
        let app = test_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/send-alert")
            .header("content-type", "application/json")
            // Ensure these match valid test IDs in the TPass test system!
            .body(Body::from(r#"{"CompId":1,"PInfo":42}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        // Adjust this assertion to whatever the REAL Tpass server actually returns
        assert_eq!(payload["message"], "alert sent");
    }

    // ... [Other tests remain the same] ...
}
