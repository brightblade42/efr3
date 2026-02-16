# Modernization Plan (Phase 2)

This plan starts after Clearview removal and PV-only backend consolidation.

## Goals

1. Upgrade dependencies to latest stable versions in controlled waves.
2. Identify breaking changes before each wave.
3. Preserve legacy TPass compatibility (insecure TLS + insecure JWT decode behavior) until downstream systems are ready.
4. Improve code reliability while minimizing behavior changes.

## Current Baseline

- Workspace crates: `fr-api`, `libfr`, `libpv`, `libtpass`, `cv-cli`
- Baseline compile state: `cargo check --workspace` passes.
- Baseline test state: `cargo test --workspace` passes (minimal test coverage).
- Current branch/commit baseline for this phase: `wild-idea` after `b6810be`.

## Progress Status

- Completed: Wave 1 (patch/minor stabilization on current major lines).
- Completed: Wave 2 (Axum/Tower major migration, including `axum::serve` startup path).
- Completed: Wave 3 (Reqwest/Base64/Jsonwebtoken major migration with legacy TPass insecure decode preserved).
- Completed: Wave 4 (`sqlx` major upgrade to `0.8.x`).
- Pending: Wave 5 cleanup/hardening.

## Current Post-Upgrade Versions

- `axum` `0.8.8`
- `axum-server` `0.8.0`
- `tower` `0.5.3`
- `tower-http` `0.6.8`
- `reqwest` `0.13.2`
- `base64` `0.22.1`
- `jsonwebtoken` `10.3.0`
- `sqlx` `0.8.6`

## Baseline Dependency Snapshot (Start of Phase 2)

- `axum`: `0.6.12` -> `0.8.8`
- `axum-server`: `0.4.7` -> `0.8.0`
- `tower-http`: `0.4.4` -> `0.6.8`
- `tower`: `0.4.13` -> `0.5.3`
- `reqwest`: `0.11.27` -> `0.13.2`
- `base64`: `0.13.1` -> `0.22.1`
- `jsonwebtoken`: `7.2.0` -> `10.3.0`
- `sqlx`: `0.7.4` -> `0.8.6` (latest stable; `0.9` is alpha)
- `thiserror`: `1.0.69` -> `2.0.18`
- `serde_with`: `2.3.3` -> `3.16.1`
- `env_logger`: `0.9.3` -> `0.11.9`
- `tokio-util`: `0.7.4` -> `0.7.18`
- `indicatif`: `0.17.3` -> `0.18.3`
- `rand`: `0.8.5` -> `0.10.0`

Notes:
- `serde`, `serde_json`, `chrono`, `regex`, `tokio`, and `tracing-subscriber` are already on current stable lines.
- `cargo tree -d` shows base64 duplication (`0.12`, `0.13`, `0.21`) mainly due to `jsonwebtoken` 7.x.

## Breaking-Change Hotspots (Known)

### Axum stack

- `fr-api/src/main.rs:252` uses `axum::Server::bind(...)` (removed in newer axum stack).
- Migration target is `tokio::net::TcpListener` + `axum::serve(listener, app)`.

### Base64 API

- `fr-api/src/main.rs:32` imports `encode_config`/`URL_SAFE` (legacy API).
- `fr-api/src/main.rs:1019` uses `base64::encode_config(...)`.
- `libpv/src/lib.rs:75`, `libfr/src/backend/paravision.rs:390`, and multiple lines in `fr-api/src/main.rs` use `base64::encode(...)`.
- Base64 >=0.21 requires engine-based API (`base64::engine::general_purpose`).

### JWT decode behavior

- `libtpass/src/tokens.rs:2` imports `dangerous_insecure_decode`.
- `libtpass/src/tokens.rs:54` decodes without signature verification.
- Jsonwebtoken 10 has API and feature changes; this path needs compatibility adapter work.

### Reqwest/TLS behavior

- `libtpass/src/api.rs:58` and `libtpass/src/api.rs:59` build client with `.danger_accept_invalid_certs(true)`.
- Reqwest 0.13 introduces changes around default TLS/features; preserve this behavior behind explicit compatibility settings.

### SQLx future incompatibility

- Current tool output reports future-incompat warnings in `sqlx-postgres 0.7.4` (never-type fallback).
- Upgrade to `sqlx 0.8.x` removes this warning class.

## Upgrade Strategy (Waves)

### Wave 0 - Safety rails and prep

- Keep behavior unchanged; no major crate bumps.
- Add/expand smoke tests around:
  - enrollment create/search/delete/reset
  - recognize + mark-attendance
  - TPass token refresh and search flow
- Add a simple migration checklist doc per wave (commands + manual endpoint checks).

Verification:
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`

### Wave 1 - Low-risk version lift

- Update patch/minor versions first where no code changes are expected:
  - `axum 0.6.20`, `tower-http 0.4.4`, `axum-server 0.4.7`
  - `reqwest 0.11.27`, `base64 0.13.1`
  - `sqlx 0.7.4`
  - `thiserror 1.0.69`, `serde_with 2.3.3`, `env_logger 0.9.3`
  - `tokio-util 0.7.18`, `indicatif 0.17.11`

Outcome:
- Stabilize on latest within current major versions before major jumps.

### Wave 2 - HTTP stack major upgrade

- Upgrade together:
  - `axum 0.8.x`
  - `tower-http 0.6.x`
  - `tower 0.5.x`
  - `axum-server 0.8.x` (or remove if TLS server path stays disabled)
- Refactor startup in `fr-api/src/main.rs` from `axum::Server::bind` to `axum::serve`.

Risk:
- Medium/high due to cross-crate compatibility in the HTTP stack.

### Wave 3 - Reqwest + Base64 + JWT major upgrade

- Upgrade:
  - `reqwest 0.13.x`
  - `base64 0.22.x`
  - `jsonwebtoken 10.x`
- Add compatibility adapter module for TPass token parsing.
- Keep insecure decode/TLS behavior available behind current compatibility defaults.

Risk:
- High (API changes + behavior-sensitive code paths).

### Wave 4 - SQLx major upgrade

- Upgrade `sqlx 0.8.x` and align feature flags.
- Re-run all DB pathways and any migration/DDL assumptions used by runtime code.

Risk:
- Medium due to query macro/runtime behavior changes.

### Wave 5 - Cleanup + hardening

- Evaluate bumping:
  - `thiserror 2.x`
  - `serde_with 3.x`
  - `env_logger 0.11.x`
  - `indicatif 0.18.x`
  - `rand 0.10.x` (optional; only if needed)
- Remove obsolete compatibility shims only when downstream systems are ready.

## Code Quality Track (Parallel)

These improvements reduce migration risk and runtime surprises:

1. Replace panic paths in runtime code (`libtpass/src/api.rs:412`).
2. Reduce `unwrap`/`expect` in request handlers and remote client flows.
3. Avoid holding mutex guards across network awaits in hot paths.
4. Split `fr-api/src/main.rs` into route modules + service layer for easier upgrades.
5. Keep `todo!()` paths untouched for now (explicitly deferred).

## Acceptance Criteria Per Wave

- All workspace crates compile: `cargo check --workspace`.
- Lints run cleanly enough for CI policy (warnings tracked separately).
- Tests + smoke checks pass for enrollment/recognition/TPass critical paths.
- No unintended API shape changes for v1/v2 endpoints unless explicitly planned.

## Operational Rule for TPass Compatibility

- Keep legacy compatibility enabled by default during modernization.
- Any future strict mode (valid certs + signature verification) must be opt-in until dependent systems are ready.
