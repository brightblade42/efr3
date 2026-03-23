//! Environment-backed application configuration for `fr-api`.
//!
//! `AppConfig` translates process environment into a strongly typed startup configuration for the
//! API server. This includes FR engine endpoints, Postgres connectivity, remote-system credentials,
//! and threshold values used to build [`libfr::types::MatchConfig`].

use crate::{env_parse, env_string, req_env_parse, req_env_string, req_env_threshold};
use libfr::types::MatchConfig;
use thiserror::Error;

/// Startup configuration assembled from environment variables.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub engine: String,
    pub remote: String,
    pub ident_addr: String,
    pub ident_port: u16,
    pub proc_addr: String,
    pub proc_port: u16,
    pub db_addr: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_pwd: String,
    pub db_name: String,
    pub db_ssl_mode: String,
    pub db_max_connections: u32,
    pub min_match: f32,
    pub min_secondary_match: f32,
    pub min_dupe_match: f32,
    pub min_quality: f32,
    pub min_acceptability: f32,
    pub max_matches_per_face: u16,
    pub port: u16,
    pub remote_url: String,
    pub remote_user: String,
    pub remote_pwd: String,
}

impl AppConfig {
    /// Load and validate the full application configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            remote: req_env_string!("EFR_REMOTE_NAME"),
            engine: req_env_string!("EFR_ENGINE_NAME"),

            //remote server
            remote_url: req_env_string!("EFR_REMOTE_URL"),
            remote_user: req_env_string!("EFR_REMOTE_USER"),
            remote_pwd: req_env_string!("EFR_REMOTE_PWD"),
            //fr backend
            ident_addr: req_env_string!("EFR_ENGINE_IDENT_ADDR"),
            ident_port: req_env_parse!("EFR_ENGINE_IDENT_PORT", u16),
            proc_addr: req_env_string!("EFR_ENGINE_PROC_ADDR"),
            proc_port: req_env_parse!("EFR_ENGINE_PROC_PORT", u16),

            //database
            db_addr: req_env_string!("EFR_DB_ADDR"),
            db_port: req_env_parse!("EFR_DB_PORT", u16),
            db_user: req_env_string!("EFR_DB_USER"),
            db_pwd: req_env_string!("EFR_DB_PWD"),
            db_name: req_env_string!("EFR_DB_NAME"),
            db_ssl_mode: env_string!("EFR_DB_SSLMODE", "disable"),
            db_max_connections: env_parse!("EFR_DB_MAX_CONN", u32, 10),
            //match config
            min_match: req_env_threshold!("EFR_MIN_MATCH"),
            min_dupe_match: req_env_threshold!("EFR_MIN_DUPE_MATCH"),
            min_secondary_match: req_env_threshold!("EFR_MIN_SECONDARY_MATCH"),
            max_matches_per_face: req_env_parse!("EFR_MAX_MATCHES_PER_FACE", u16),
            min_quality: req_env_threshold!("EFR_MIN_QUALITY"),
            min_acceptability: req_env_threshold!("EFR_MIN_ACCEPTABILITY"),

            //main api
            port: env_parse!("EFR_SERVER_PORT", u16, 3000),
        })
    }
}

/// Reduce the full application config down to the matching thresholds used by `libfr`.
impl From<&AppConfig> for MatchConfig {
    fn from(c: &AppConfig) -> Self {
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

/// Parse a threshold as either a ratio (`0.95`) or whole-number percent (`95`).
pub fn parse_threshold(key: &str, raw: &str) -> Result<f32, ConfigError> {
    // 1. Remove ANY whitespace (including \r, \n, tabs)
    // 2. Remove quotes just in case
    let sanitized: String =
        raw.chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect();

    let parsed = sanitized.parse::<f32>().map_err(|e| ConfigError::Invalid {
        key: key.into(),
        value: raw.to_string(),
        message: format!("Parse failure: {}", e),
    })?;

    let normalized = if parsed > 1.0 { parsed / 100.0 } else { parsed };
    // DEBUG LOG
    //enforce range.
    if !(0.0..=1.0).contains(&normalized) {
        return Err(ConfigError::Invalid {
            key: key.into(),
            value: raw.to_string(),
            message: "threshold must be between 0.0 and 1.0, or between 0 and 100".into(),
        });
    }

    Ok(normalized)
}

/// Configuration-loading errors surfaced during startup.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("🔴 missing require environment var: {0}")]
    Missing(String),

    #[error("🔴 invalid env var {key}={value:?}: {message}")]
    Invalid { key: String, value: String, message: String },
}
