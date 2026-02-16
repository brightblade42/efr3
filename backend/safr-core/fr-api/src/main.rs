mod attendance_handlers;
mod enrollment_handlers;
mod errors;
mod extractors;
mod profile_handlers;
mod recognition_handlers;
mod tpass_handlers;
mod types;
mod v1_handlers;
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
use libfr::backend::paravision::PVBackend;
use libfr::{
    backend::{FRBackend, MatchConfig},
    remote::Remote,
};

use libtpass::api::TPassClient;

type WResult<T> = Result<T, AppError>;

//TODO: figure out how to actually swap out the backend. I need to refer to the
//FRBackend trait but the compiler yells at me! I have to use the concrete type.
#[derive(Clone)]
struct AppState {
    backend: Arc<dyn FRBackend + Send + Sync>,
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
    _min_quality: f32,
    port: u16,
}
impl Config {
    fn new() -> Self {
        //mostly precautionary, env vars are provided by docker-compose files
        let dev_url = "192.168.0.204";
        //let dev_url = "174.51.11.19";
        let min_match = env::var("MIN_MATCH").unwrap_or("0.95".to_string());
        let min_quality = env::var("MIN_QUALITY").unwrap_or("0.80".to_string());
        let min_dupe_match = env::var("MIN_DUPE_MATCH").unwrap_or("0.98".to_string());
        let use_tls = env::var("USE_TLS").unwrap_or("false".to_string());
        let port = env::var("FRAPI_PORT").unwrap_or("3000".to_string());

        Self {
            pv_ident_url: env::var("PV_IDENT_URL").unwrap_or(format!("{}:8080", dev_url)),
            pv_proc_url: env::var("PV_PROC_URL").unwrap_or(format!("{}:8081/v6", dev_url)),
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
            _cert_dir: env::var("CERT_DIR").unwrap_or("/cert".to_string()), //should we keep env?
            _use_tls: use_tls.parse().unwrap_or(false),

            min_match: min_match.parse().unwrap_or(0.95),
            _min_quality: min_quality.parse().unwrap_or(0.80),

            min_dupe_match: min_dupe_match.parse().unwrap_or(0.98),
            port: port.parse().unwrap_or(3000),
        }
    }
}

type Backend = Arc<dyn FRBackend + Send + Sync>; //quit the noise a touch

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

    let tpass_client = Arc::new(TPassClient::new(None));
    let tp_remote: Arc<dyn Remote> = tpass_client.clone();

    let backend = Arc::new(PVBackend::new(
        config.pv_proc_url.clone(),
        config.pv_ident_url.clone(),
        db_pool,
        Some(tp_remote),
    )) as Backend;

    info!("BACKEND: pv");

    let app_state = AppState {
        backend,
        tpass_client,
        config: config.clone(), //some tpass specific calls
    };

    let api_v2_routes = Router::new()
        .route(
            "/enrollment/metadata",
            get(enrollment_handlers::get_enrollment_metadata)
                .post(enrollment_handlers::create_collection),
        )
        .route(
            "/enrollment/roster",
            get(enrollment_handlers::get_enrollment_roster),
        )
        .route(
            "/enrollment/create",
            post(enrollment_handlers::create_enrollment),
        )
        .route(
            "/enrollment/search",
            post(enrollment_handlers::search_enrollment),
        )
        .route(
            "/enrollment/delete",
            post(enrollment_handlers::delete_enrollment),
        )
        .route(
            "/enrollment/reset",
            post(enrollment_handlers::reset_enrollments),
        )
        .route(
            "/enrollment/errlog",
            post(enrollment_handlers::get_enrollment_errlog),
        )
        .route("/create-profile", post(profile_handlers::create_profile))
        .route("/edit-profile", post(profile_handlers::edit_profile))
        .route("/detect", post(recognition_handlers::detect_image)) //detect, bbox.
        .route(
            "/detect_spoof",
            post(recognition_handlers::detect_spoof_image),
        ) //detect, bbox.
        //.route("/detect_embed", post(detect_image_embed)) //detect, bbox + embeddngs
        .route("/recognize", post(recognition_handlers::recognize))
        //a combo on recognition and notifying remote of building entrance / exit.
        //NOTE: was called recognize-faces-b64, remember for when camera app breaks lol
        .route(
            "/mark-attendance",
            post(attendance_handlers::mark_attendance),
        );

    let tpass_routes = Router::new()
        .route("/get-companies", get(tpass_handlers::get_tpass_companies))
        .route(
            "/get-client-types",
            get(tpass_handlers::get_tpass_client_types),
        )
        .route(
            "/get-status-types",
            get(tpass_handlers::get_tpass_status_types),
        )
        .route("/search", post(tpass_handlers::search_tpass))
        .fallback(fallback1);

    //V1 backport
    let api_v1_routes = Router::new()
        //NOTE: DEPRECATED
        .route(
            "/recognize-faces-b64",
            post(attendance_handlers::mark_attendance),
        )
        .route("/recognize-faces", post(recognition_handlers::recognize))
        .route("/recognize", post(v1_handlers::recognize_v1))
        .route(
            "/enrollment/create",
            post(v1_handlers::create_enrollment_v1),
        )
        .route(
            "/enrollment/delete",
            post(v1_handlers::delete_enrollment_v1),
        )
        .route("/enrollment/add-face", post(v1_handlers::add_face_v1))
        .route("/enrollment/delete-face", post(v1_handlers::delete_face_v1))
        .route("/get-identity", post(v1_handlers::get_faces_v1))
        .route("/create-profile", post(profile_handlers::create_profile))
        .route("/edit-profile", post(profile_handlers::edit_profile))
        .route("/send-alert", post(tpass_handlers::send_fr_alert));

    let app = Router::new()
        .nest("/fr/v2", api_v2_routes)
        .nest("/fr", api_v1_routes)
        .nest("/tpass", tpass_routes)
        //This is how we serve our static svelte files.
        .nest_service("/_app", ServeDir::new("./app/_app"))
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    //   .allow_credentials(true)
                    .allow_methods([Method::GET, Method::POST])
                    .allow_origin(Any),
            ),
        )
        // .fallback_service(get_service(ServeFile::new("./app/200.html")).handle_error(
        //     |_| async move {
        //         (
        //             StatusCode::INTERNAL_SERVER_ERROR,
        //             "couldn't load main index file",
        //         )
        //     },
        // ))
        .with_state(app_state);

    //set up v1 endpoints. these differe mainly in the shape of the returned data.
    //the api versioning may at some point be handled by a reverse proxy that will forward to
    //the proper service version. That way, the real endpoints never change.

    //TODO: setup for http redirect.
    // if config.use_tls {
    //     let addr = SocketAddr::from(([0, 0, 0, 0], 443));
    //
    //     //prod
    //     let key_file = "/cert/fr-api-key.pem";
    //     let cert_file = "/cert/fr-api-cert.pem";
    //     //setup tls
    //     let tls_conf = RustlsConfig::from_pem_file(cert_file, key_file)
    //         .await
    //         .unwrap();
    //
    //     info!("listening on {}", addr);
    //     axum_server::bind_rustls(addr, tls_conf)
    //         .serve(app.into_make_service())
    //         .await
    //         .unwrap();
    // } else {
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
        }
    }
}
//we're not using this one currently
/*
async fn detect_image_embed(State(app_state): State<AppState>, multipart: Multipart) -> WResult<Json<Value>> {

    let img_data = extract_image_data(multipart).await?;
    let res  = app_state.backend.detect_image_embed(img_data.image.unwrap()).await?;
    Ok(Json(res))
}
 */
