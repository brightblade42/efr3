mod attendance_handlers;
mod enrollment_handlers;
mod errors;
mod extractors;
mod fr_service;
mod profile_handlers;
mod recognition_handlers;
mod runtime;
mod tpass_handlers;
use axum::http::{Method, StatusCode};
//use axum_server::tls_rustls::RustlsConfig;
use tracing_subscriber::EnvFilter;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::{error, info};

//use tokio::sync::Mutex;
use crate::errors::AppError;
use crate::fr_service::FRService;
use crate::runtime::{FREngine, RemoteRuntime};
use libfr::repo::SqlxFrRepository;
use libfr::{backend::MatchConfig, utils};

use libtpass::api::TPassClient;

type WResult<T> = Result<T, AppError>;

// Backend/remote are selected once at startup via env defaults.
#[derive(Clone)]
struct AppState {
    fr_service: Arc<FRService>,
    fr_repo: Arc<SqlxFrRepository>,
    tpass_client: Arc<TPassClient>,
    config: Config,
}

#[derive(Clone)]
struct Config {
    pv_ident_url: String,
    pv_proc_url: String,
    db_addr: String,
    db_port: String,
    db_user: String,
    db_pwd: String,
    db_name: String,
    db_ssl_mode: String,
    db_max_connections: u32,
    _cert_dir: String,
    _use_tls: bool,
    min_match: f32,
    min_dupe_match: f32,
    min_quality: f32,
    min_acceptability: f32,
    port: u16,
}
impl Config {
    fn parse_score_threshold(raw: &str, fallback: f32) -> f32 {
        raw.trim()
            .parse::<f32>()
            .map(utils::normalize_score_threshold)
            .unwrap_or(fallback)
    }

    fn new() -> Self {
        //mostly precautionary, env vars are provided by docker-compose files
        let dev_url = "100.79.241.8";
        let min_match = env::var("MIN_MATCH").unwrap_or("0.95".to_string());
        let min_quality = env::var("MIN_QUALITY").unwrap_or("0.80".to_string());
        let min_acceptability =
            env::var("MIN_ACCEPTABILITY").unwrap_or_else(|_| min_quality.clone());
        let min_dupe_match = env::var("MIN_DUPE_MATCH").unwrap_or("0.98".to_string());
        let use_tls = env::var("USE_TLS").unwrap_or("false".to_string());
        let port = env::var("FRAPI_PORT").unwrap_or("3000".to_string());

        Self {
            //TODO: remove urls for http api since we've moved to gRPC
            pv_ident_url: env::var("PV_IDENT_URL").unwrap_or(format!("http://{}:5656", dev_url)),
            pv_proc_url: env::var("PV_PROC_URL").unwrap_or(format!("http://{}:50051", dev_url)),

            //TODO: we don't use the term SAFR anymore, it's EYEFR
            db_addr: env::var("SAFR_DB_ADDR").unwrap_or("localhost".to_string()),
            db_port: env::var("SAFR_DB_PORT").unwrap_or("5433".to_string()),
            db_user: env::var("SAFR_DB_USER").unwrap_or("admin".to_string()),
            db_pwd: env::var("SAFR_DB_PWD").unwrap_or("admin".to_string()),
            db_name: env::var("SAFR_DB_NAME").unwrap_or("safr".to_string()),
            db_ssl_mode: env::var("SAFR_DB_SSLMODE").unwrap_or("disable".to_string()),
            db_max_connections: env::var("SAFR_DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(5),

            //NOTE: this is currently handled by a reverse proxy.
            _cert_dir: env::var("CERT_DIR").unwrap_or("/cert".to_string()), //should we keep env?
            _use_tls: use_tls.parse().unwrap_or(false),

            //NOTE: seems unneeded since we fallback on a known good value when parsing env vars
            min_match: Self::parse_score_threshold(&min_match, 0.95),
            min_quality: min_quality.parse().unwrap_or(0.80),
            min_acceptability: min_acceptability.parse().unwrap_or(0.80),

            min_dupe_match: Self::parse_score_threshold(&min_dupe_match, 0.98),
            port: port.parse().unwrap_or(3000),
        }
    }
}

fn api_v2_routes() -> Router<AppState> {
    Router::new()
        .route("/enrollment/create", post(enrollment_handlers::create_enrollment))
        .route("/enrollment/search", post(enrollment_handlers::search_enrollment))
        .route("/enrollment/delete", post(enrollment_handlers::delete_enrollment))
        //TODO
        .route("/enrollment/add-face", post(enrollment_handlers::add_face))
        .route("/enrollment/delete-face", post(enrollment_handlers::delete_face))
        //TODO: not sure about this.
        //.route("/get-identity", post(enrollment_handlers::get_face_info))
        //PROFILE interacts with REMOTE
        .route("/create-profile", post(profile_handlers::create_profile))
        .route("/edit-profile", post(profile_handlers::edit_profile))
        .route("/send-alert", post(tpass_handlers::send_fr_alert))
        //TODOD: this name needs to be better.
        .route("/mark-attendance", post(attendance_handlers::mark_attendance))
        //NOTE: deprecated in favor of liveness-check, clearer name
        //TODO: delete validate-image after liveness demo is complete
        .route("/validate-image", post(recognition_handlers::liveness_check))
        //NOTE: liveness-check, does liveness and includes quality
        .route("/liveness-check", post(recognition_handlers::liveness_check))
        //just the quality. validate is a more verbose version
        .route("/quality-check", post(recognition_handlers::quality_check))
        .route("/detect", post(recognition_handlers::detect_faces)) //detect, bbox.
        .route("/recognize", post(recognition_handlers::recognize))
        //a combo on recognition and notifying remote of building entrance / exit.
        //DELETE
        //NOTE: this is a very dangerous function. maybe we block it.
        .route("/enrollment/reset", post(enrollment_handlers::reset_enrollments))
        //TODO: implement or discard
        .route("/enrollment/errlog", post(enrollment_handlers::get_enrollment_errlog))
        .route("/enrollment/metadata", get(enrollment_handlers::get_enrollment_metadata))
        //gets all the enrollments 100 max atm
        .route("/enrollment/roster", get(enrollment_handlers::get_enrollment_roster))
}

fn tpass_routes() -> Router<AppState> {
    Router::new()
        .route("/get-companies", get(tpass_handlers::get_tpass_companies))
        .route("/get-client-types", get(tpass_handlers::get_tpass_client_types))
        .route("/get-status-types", get(tpass_handlers::get_tpass_status_types))
        //TODO: is this  something we use in production or was this just for testing?
        //might be better to elimate for security reasons. A tpass passthrough function is probably
        // not the best idea.
        .route("/search", post(tpass_handlers::search_tpass))
        .fallback(fallback1)
}

//TODO: add v1 routes as needed for compat
//        //v1 endpoint for cam app, deprecated
//.route("/recognize-faces-b64", post(attendance_handlers::mark_attendance))

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        //        .with_max_level(Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env()) //do I even need this? I may if I want to reduce tracing output as an optimization in prod
        .init();

    info!("starting web server for glorious victories!");

    let config = Config::new();
    let db_conn = format!(
        "postgresql://{}:{}@{}:{}/{}?sslmode={}",
        config.db_user.clone(),
        config.db_pwd.clone(),
        config.db_addr.clone(),
        config.db_port.clone(),
        config.db_name.clone(),
        config.db_ssl_mode.clone(),
    );

    let db_pool = match PgPoolOptions::new()
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

    //Arc'em up!
    let tpass_client = Arc::new(TPassClient::new(None));
    let fr_repo = Arc::new(SqlxFrRepository::new(db_pool.clone()));
    let fr_remote_env = env::var("FR_REMOTE").ok();
    let fr_backend_env = env::var("FR_BACKEND").ok();

    //NOTE: not sure i understand the purpose of this
    let remote = match RemoteRuntime::from_env(fr_remote_env.clone(), tpass_client.clone()) {
        Ok(remote) => remote,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    let remote_name = remote.name();
    let remote = Arc::new(remote);

    let fr_engine = match FREngine::from_env(
        fr_backend_env.clone(),
        config.pv_proc_url.clone(),
        config.pv_ident_url.clone(),
        db_pool.clone(),
    ) {
        Ok(fr_engine) => fr_engine,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    info!(
        "startup env FR_BACKEND={} FR_REMOTE={}",
        fr_backend_env.as_deref().unwrap_or("<unset>"),
        fr_remote_env.as_deref().unwrap_or("<unset>"),
    );

    info!("BACKEND: {}", fr_engine.name());
    info!("REMOTE: {}", remote_name);

    let fr_service = Arc::new(FRService::new(Arc::new(fr_engine), remote, fr_repo.clone()));

    let app_state = AppState {
        fr_service,
        fr_repo,
        tpass_client,
        config: config.clone(), //some tpass specific calls
    };

    let app =
        Router::new()
            .nest("/fr/v2", api_v2_routes())
            .nest("/tpass", tpass_routes())
            //NOTE: i think we moved site serving out of here and up to the rev proxy
            .nest_service("/_app", ServeDir::new("./app/_app"))
            .layer(ServiceBuilder::new().layer(
                CorsLayer::new().allow_methods([Method::GET, Method::POST]).allow_origin(Any),
            ))
            .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("listening on {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("failed to bind listener on {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("server error: {}", e);
    }
    //}
}

async fn fallback1() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "This is not cool bruh..")
}

impl From<&Config> for MatchConfig {
    fn from(c: &Config) -> Self {
        Self {
            min_match: c.min_match,
            min_dupe_match: c.min_dupe_match,
            top_n: 2,
            top_n_min_match: 0.80,
            min_quality: c.min_quality,
            min_acceptability: c.min_acceptability,
            include_details: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::{routing::post, Json};
    use libtpass::config::TPassConf;
    use serde_json::{json, Value};
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state_with_tpass_url(tpass_url: &str) -> AppState {
        let db_pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(200))
            .connect_lazy("postgresql://admin:admin@127.0.0.1:9/identity?sslmode=disable")
            .expect("lazy db pool should build");

        let tpass_client = Arc::new(TPassClient::new(Some(TPassConf {
            url: tpass_url.to_string(),
            user: "test-user".to_string(),
            pwd: "test-pwd".to_string(),
        })));

        let remote = Arc::new(
            RemoteRuntime::from_env(Some("tpass".to_string()), tpass_client.clone())
                .expect("remote runtime should initialize"),
        );

        let fr_engine = FREngine::mock();

        let fr_repo = Arc::new(SqlxFrRepository::new(db_pool));
        let fr_service = Arc::new(FRService::new(Arc::new(fr_engine), remote, fr_repo.clone()));

        AppState { fr_service, fr_repo, tpass_client, config: Config::new() }
    }

    fn test_app() -> Router {
        test_app_with_tpass_url("https://example.invalid/")
    }

    fn test_app_with_tpass_url(tpass_url: &str) -> Router {
        Router::new()
            .nest("/fr/v2", api_v2_routes())
            .with_state(test_state_with_tpass_url(tpass_url))
    }

    fn multipart_image_request(uri: &str) -> Request<Body> {
        let boundary = "X-BOUNDARY";
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

    fn mock_jwt_token() -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD
            .encode(r#"{"Name":"tester","Role":"admin","CCode":"1","exp":4102444800}"#);

        format!("{}.{}.e30", header, claims)
    }

    async fn spawn_mock_tpass_server() -> (String, tokio::task::JoinHandle<()>) {
        async fn token_handler() -> Json<Value> {
            Json(json!({"token": mock_jwt_token()}))
        }

        async fn send_alert_handler() -> Json<Value> {
            Json(json!({"ok": true}))
        }

        let app = Router::new()
            .route("/api/token", post(token_handler))
            .route("/api/notification/sendalert", post(send_alert_handler));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock tpass listener");
        let addr = listener.local_addr().expect("mock tpass local addr");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock tpass server should run");
        });

        (format!("http://{}/", addr), handle)
    }

    #[tokio::test]
    async fn add_face_requires_fr_id_query_param() {
        let app = test_app();
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
        let app = test_app();
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
        let app = test_app();
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
        let app = test_app();
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
    async fn add_face_happy_path_returns_faces_payload() {
        let app = test_app();
        let req = multipart_image_request("/fr/v2/enrollment/add-face?fr_id=mock-fr-id");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        let faces = payload.get("faces").and_then(Value::as_array).expect("faces array");
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0]["id"], "mock-face-id");
    }

    #[tokio::test]
    async fn delete_face_happy_path_returns_delete_payload() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/delete-face")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"fr_id":"mock-fr-id","face_id":"mock-face-id"}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["rows_affected"], 1);
        assert_eq!(payload["fr_id"], "mock-fr-id");
        assert_eq!(payload["face_id"], "mock-face-id");
    }

    #[tokio::test]
    async fn get_identity_happy_path_returns_faces_payload() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/get-identity")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"fr_id":"mock-fr-id"}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        let faces = payload.get("faces").and_then(Value::as_array).expect("faces array");
        assert_eq!(faces.len(), 1);
        assert_eq!(payload["total_size"], 1);
    }

    #[tokio::test]
    async fn send_alert_happy_path_returns_message_payload() {
        let (tpass_url, handle) = spawn_mock_tpass_server().await;
        let app = test_app_with_tpass_url(&tpass_url);

        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/send-alert")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"CompId":1,"PInfo":42}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["message"], "alert sent");

        handle.abort();
    }

    #[tokio::test]
    async fn create_enrollment_missing_details_returns_standard_error_shape() {
        let app = test_app();
        let req = multipart_enrollment_request("/fr/v2/enrollment/create", true, false, false);

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["code"], 0);
        assert!(payload["message"].is_string());
    }

    #[tokio::test]
    async fn create_enrollment_missing_ext_id_returns_standard_error_shape() {
        let app = test_app();
        let req = multipart_enrollment_request("/fr/v2/enrollment/create", true, true, false);

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["code"], 1050);
        assert!(payload["message"].is_string());
    }

    #[tokio::test]
    async fn delete_enrollment_empty_fr_id_returns_standard_error_shape() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/delete")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"fr_id":""}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["code"], 0);
        assert!(payload["message"].is_string());
    }

    #[tokio::test]
    async fn search_enrollment_db_failure_returns_standard_error_shape() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/search")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"last_name":"User"}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["code"], 1061);
        assert!(payload["message"].is_string());
        assert!(payload["details"].is_object());
    }

    #[tokio::test]
    async fn delete_enrollment_db_failure_returns_standard_error_shape() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/fr/v2/enrollment/delete")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"fr_id":"mock-fr-id"}"#))
            .expect("request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let payload = response_json(resp).await;
        assert_eq!(payload["code"], 1060);
        assert!(payload["message"].is_string());
        assert!(payload["details"].is_object());
    }
}
//we're not using this one currently
/*
async fn detect_image_embed(State(app_state): State<AppState>, multipart: Multipart) -> WResult<Json<Value>> {

    let img_data = extract_image_data(multipart).await?;
    let res  = app_state.fr_service.detect_image_embed(img_data.image.unwrap()).await?;
    Ok(Json(res))
}
 */
