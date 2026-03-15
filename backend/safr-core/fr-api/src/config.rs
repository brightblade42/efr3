use crate::{env_parse, env_string, req_env_parse, req_env_string, req_env_threshold};
use std::fmt;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub pv_ident_url: String,
    pub pv_proc_url: String,
    pub db_addr: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_pwd: String,
    pub db_name: String,
    pub db_ssl_mode: String,
    pub db_max_connections: u32,
    pub min_match: f32,
    pub min_dupe_match: f32,
    pub min_quality: f32,
    pub min_acceptability: f32,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            pv_ident_url: req_env_string!("PV_IDENT_URL"),
            pv_proc_url: req_env_string!("PV_PROC_URL"),
            db_addr: req_env_string!("SAFR_DB_ADDR"),
            db_port: req_env_parse!("SAFR_DB_PORT", u16),
            db_user: req_env_string!("SAFR_DB_USER"),
            db_pwd: req_env_string!("SAFR_DB_PWD"),
            db_name: req_env_string!("SAFR_DB_NAME"),
            db_ssl_mode: env_string!("SAFR_DB_SSLMODE", "disable"),
            db_max_connections: env_parse!("SAFR_DB_MAX_CONNECTIONS", u32, 10),
            min_match: req_env_threshold!("MIN_MATCH"),
            min_dupe_match: req_env_threshold!("MIN_DUPE_MATCH"),
            min_quality: req_env_threshold!("MIN_QUALITY"),
            min_acceptability: req_env_threshold!("MIN_ACCEPTABILITY"),
            port: env_parse!("FRAPI_PORT", u16, 3000),
        })
    }
}
pub fn parse_threshold(key: &str, raw: &str) -> Result<f32, ConfigError> {
    let parsed = raw.trim().parse::<f32>().map_err(|e| ConfigError::Invalid {
        key: key.into(),
        value: raw.to_string(),
        message: e.to_string(),
    })?;

    //NOTE: a little aggressive.
    if !parsed.is_finite() {
        return Err(ConfigError::Invalid {
            key: key.into(),
            value: raw.to_string(),
            message: "threshold must be a finite number".into(),
        });
    }

    let normalized = if parsed > 1.0 { parsed / 100.0 } else { parsed };

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

#[derive(Debug, Clone)]
pub enum ConfigError {
    Missing(String),
    Invalid { key: String, value: String, message: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(key) => write!(f, "missing required env var {key}"),
            Self::Invalid { key, value, message } => {
                write!(f, "invalid env var {key}={value:?}: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}
