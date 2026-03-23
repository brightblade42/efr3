//! `libfr` is the core orchestration crate behind the SAFR backend.
//!
//! It sits between three distinct concerns:
//! - an FR engine implementation, currently Paravision gRPC
//! - a remote system of record for people and attendance, currently TPass
//! - local Postgres persistence for profile snapshots and operational logs
//!
//! The crate exposes a small set of modules that define those boundaries:
//! - [`dispatch`] selects concrete FR and remote implementations at runtime
//! - [`service`] coordinates enrollment, recognition, attendance, and logging flows
//! - [`repo`] persists local enrollment metadata and audit-style records
//! - [`pv`] contains the Paravision integration
//! - [`tpass`] contains the TPass integration
//! - [`types`] defines the main transport and domain types used across layers
//! - [`errors`] defines the domain error surface shared by callers
//!
//! For internal engineers, the best entry points are [`service::FRService`],
//! [`dispatch::FRBackend`], [`dispatch::AssetStore`], and [`types::MatchConfig`].
//!
//! Runtime model:
//! 1. `fr-api` parses HTTP input and converts it into `libfr` request types.
//! 2. [`service::FRService`] validates input and coordinates backend/repo/remote calls.
//! 3. [`dispatch::FRDispatcher`] forwards FR operations to the configured engine.
//! 4. [`dispatch::AssetDispatcher`] forwards profile and attendance operations to the remote.
//! 5. [`repo::SqlxFrRepository`] stores local snapshots, logs, and summary metadata.
//!
//! The crate intentionally favors typed request/response models over generic JSON where the
//! payload shape is stable, but some compatibility layers still carry `serde_json::Value`.
//!
//! Environment-sensitive behavior such as network endpoints and thresholds is configured in
//! `fr-api`; this crate assumes dependencies are already constructed and injected.
//!
//! ## Documentation Map
//!
//! - Architecture and engineering notes: `backend/safr-core/docs/libfr.md`
//! - System overview: `backend/safr-core/docs/architecture.md`
//! - Runtime flows: `backend/safr-core/docs/runtime-flow.md`

#[macro_use]
mod macros;
pub mod dispatch;
///lib.rs
///The main library for interacting with an FR engine and a remote management system.
///An FR Engine is a backend that handles the analysis of images to find faces
///and provide information about them, and the identification of those faces.
///The remote management system is a 3rd party or local system for managing people (vms/cms/erp)
///It's purpose is to store identifying information and images of people.
///We keep that separate to keep a privacy barrier. We store as little personal information
///as possible and delegate to another service for that.
///libfr combines the 2 pillars (FREngine and Remote) into a unified library
///it coordinates between the two, logs information about system events.
///it provides a system on top to facilitate the managment of enrollments.
///Enrollments represent an identity that has been entered into the system by providing
///an image and some minimal identitifying information , as little as an id that can be used to
///query the remote for personal details as necessary for returning to a user/program at the time
///of a positive facial recognition event.
pub mod errors;
pub mod pv;
pub mod repo;
pub mod service;
pub mod tpass;
mod tpass_asset_store;
pub mod types;

pub mod utils {

    pub fn round(x: f64, decimals: u32) -> f64 {
        let y = 10i32.pow(decimals) as f64;
        (x * y).round() / y
    }

    pub fn roundf32(x: f32, decimals: u32) -> f32 {
        let y = 10i32.pow(decimals) as f32;
        (x * y).round() / y
    }

    pub fn score_to_percentage(score: f32) -> f32 {
        roundf32(score * 100.0, 2)
    }

    /// Accept either ratio-style (`0.98`) or percent-style (`98.0`) thresholds.
    pub fn normalize_score_threshold(threshold: f32) -> f32 {
        let raw = if threshold > 1.0 { threshold / 100.0 } else { threshold };

        raw.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{types::PossibleMatch, utils};
    use serde_json::json;

    #[test]
    fn possible_match_serializes_score_field() {
        let possible_match = PossibleMatch {
            fr_id: "i_test".to_string(),
            score: 0.99,
            score_pct: 99.0,
            ext_id: "123".to_string(),
            details: None,
        };

        let value = serde_json::to_value(possible_match).expect("serialize possible match");
        assert!(value.get("score").is_some());
        assert_eq!(value.get("score_pct").and_then(|value| value.as_f64()), Some(99.0));
        assert!(value.get("confidence").is_none());
    }

    #[test]
    fn possible_match_deserializes_confidence_alias() {
        let value = json!({
            "fr_id": "i_test",
            "confidence": 0.75,
            "ext_id": "456",
            "details": null
        });

        let mut possible_match: PossibleMatch =
            serde_json::from_value(value).expect("deserialize possible match from confidence");
        assert!((possible_match.score - 0.75).abs() < f32::EPSILON);
        possible_match.refresh_score_percentage();
        assert_eq!(possible_match.score_pct, 75.0);
    }

    #[test]
    fn score_to_percentage_is_rounded() {
        assert_eq!(utils::score_to_percentage(0.98765), 98.77);
    }

    #[test]
    fn normalize_score_threshold_accepts_ratio_and_percent() {
        assert_eq!(utils::normalize_score_threshold(0.98), 0.98);
        assert_eq!(utils::normalize_score_threshold(98.0), 0.98);
    }
}
