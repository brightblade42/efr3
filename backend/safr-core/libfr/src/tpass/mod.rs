#![allow(non_snake_case)]
//! TPass integration modules.
//!
//! TPass acts as the remote system of record for person profiles, attendance events, and alerting.
//! The submodules here contain the HTTP client, configuration parsing, token handling, error types,
//! and request/response models used by the rest of the workspace.

pub mod api;
pub mod config;
pub mod errors;
pub mod tokens;
pub mod types;
