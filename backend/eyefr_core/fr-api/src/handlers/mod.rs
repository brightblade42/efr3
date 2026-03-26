//! HTTP handler modules grouped by route family.
//!
//! - [`attendance_handlers`] coordinates recognition plus attendance side effects
//! - [`enrollment_handlers`] owns enrollment CRUD, roster, metadata, and compatibility routes
//! - [`profile_handlers`] wraps remote profile creation and editing flows
//! - [`recognition_handlers`] exposes quality, liveness, detect, and recognize operations
//! - [`tpass_handlers`] provides internal helper routes for direct TPass access

pub mod attendance_handlers;
pub mod enrollment_handlers;
pub mod profile_handlers;
pub mod recognition_handlers;
pub mod tpass_handlers;
