/// Required string env var.
/// Fails if the key is missing.
#[macro_export]
macro_rules! req_env_string {
    ($key:literal) => {
        ::std::env::var($key).map_err(|_| crate::config::ConfigError::Missing($key.into()))?
    };
}

/// Optional string env var with fallback.
/// Uses fallback only when the key is missing.
#[macro_export]
macro_rules! env_string {
    ($key:literal, $default:expr) => {
        ::std::env::var($key).unwrap_or_else(|_| ($default).into())
    };
}

/// Required parsed env var.
/// Fails if the key is missing or present but invalid.
#[macro_export]
macro_rules! req_env_parse {
    ($key:literal, $t:ty) => {{
        let raw =
            ::std::env::var($key).map_err(|_| crate::config::ConfigError::Missing($key.into()))?;

        raw.trim().parse::<$t>().map_err(|e| crate::config::ConfigError::Invalid {
            key: $key.into(),
            value: raw,
            message: e.to_string(),
        })?
    }};
}

/// Optional parsed env var with fallback.
/// Uses fallback only when the key is missing.
/// Still fails if the key is present but invalid.
#[macro_export]
macro_rules! env_parse {
    ($key:literal, $t:ty, $default:expr) => {{
        match ::std::env::var($key) {
            Ok(raw) => {
                raw.trim().parse::<$t>().map_err(|e| crate::config::ConfigError::Invalid {
                    key: $key.into(),
                    value: raw,
                    message: e.to_string(),
                })?
            }
            Err(_) => $default,
        }
    }};
}

/// Required threshold env var.
/// Fails if the key is missing or invalid.
#[macro_export]
macro_rules! req_env_threshold {
    ($key:literal) => {{
        let raw =
            ::std::env::var($key).map_err(|_| crate::config::ConfigError::Missing($key.into()))?;

        crate::config::parse_threshold($key, &raw)?
    }};
}

/// Optional threshold env var with fallback.
/// Uses fallback only when the key is missing.
/// Still fails if the key is present but invalid.
#[macro_export]
macro_rules! env_threshold {
    ($key:literal, $default:expr) => {{
        match ::std::env::var($key) {
            Ok(raw) => crate::config::parse_threshold($key, &raw)?,
            Err(_) => $default,
        }
    }};
}
