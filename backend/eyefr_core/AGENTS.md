# AGENTS.md

Guidance for agentic coding agents working in the Rust workspace at `backend/safr-core`.

## Scope

- This file applies to `backend/safr-core` and its workspace crates.
- Workspace members: `fr-api`, `cv-cli`, `libfr`.
- Primary stack: Rust 2021, Tokio, Axum, SQLx/Postgres, Reqwest, Tonic gRPC.
- Make focused changes; do not do broad cleanup unless requested.

## Rule Files (Cursor / Copilot)

- Checked for Cursor rules in `/Users/ryan/projects/efr3/safr/.cursor/rules/` and `/Users/ryan/projects/efr3/safr/.cursorrules`.
- Checked for Copilot rules in `/Users/ryan/projects/efr3/safr/.github/copilot-instructions.md`.
- No Cursor or Copilot rule files were present when this file was updated.
- If any of those files are added later, treat them as higher-priority instructions.

## Workspace Layout

- `fr-api`: Axum HTTP service and route wiring.
- `cv-cli`: operator CLI for indexing and enrollment flows.
- `libfr`: core domain/service layer, backend dispatch, repo layer, Paravision and TPass clients.
- `libfr/tests`: live/integration-style tests for Paravision gRPC and repo behavior.
- `examples/`: curl/shell examples for API endpoints.
- `db/schema/live/`: checked-in SQL schema references and refresh helpers.

## Working Directory

- Run Cargo commands from `backend/safr-core` unless noted otherwise.
- Prefer crate-scoped commands for faster iteration when only one crate changed.

## Build Commands

- Build workspace: `cargo build --workspace`
- Build release: `cargo build --workspace --release`
- Build one crate: `cargo build -p fr-api`
- Fast typecheck: `cargo check --workspace`
- Fast typecheck one crate: `cargo check -p libfr`

## Format / Lint Commands

- Format everything: `cargo fmt --all`
- Check formatting only: `cargo fmt --all --check`
- Lint workspace: `cargo clippy --workspace --all-targets`
- Lint one crate: `cargo clippy -p fr-api --all-targets`

## Test Commands

- Run all tests: `cargo test --workspace`
- Run one crate: `cargo test -p libfr`
- Run doc tests: `cargo test --workspace --doc`
- List tests in a crate before targeting one: `cargo test -p libfr -- --list`

## Single-Test Recipes

- Run one unit test by substring: `cargo test -p libfr score_to_percentage`
- Run one exact unit test: `cargo test -p libfr score_to_percentage_is_rounded -- --exact --nocapture`
- Run one `fr-api` test: `cargo test -p fr-api add_face_requires_fr_id_query_param -- --exact --nocapture`
- Run one integration test target file: `cargo test -p libfr --test pv_identity_grpc_live`
- Run one exact test inside an integration target: `cargo test -p libfr --test pv_identity_grpc_live live_identity_grpc_health_check -- --exact --nocapture`
- Run ignored live tests explicitly: `cargo test -p libfr --test pv_identity_grpc_live live_identity_grpc_health_check -- --ignored --exact --nocapture`
- Some tests require external services or env vars; prefer exact test selection to avoid accidental live calls.

## Run Commands

- Start API: `cargo run -p fr-api`
- Run CLI help: `cargo run -p cv-cli -- --help`
- Example CLI call: `cargo run -p cv-cli -- --url http://localhost:3000 reset`

## Runtime / Env Notes

- `fr-api` loads env via `dotenvy` and `AppConfig::from_env()`.
- Common API env vars include `FRAPI_PORT`, `PV_IDENT_URL`, `PV_PROC_URL`, `SAFR_DB_ADDR`, `SAFR_DB_PORT`, `MIN_MATCH`, `MIN_DUPE_MATCH`, `MIN_QUALITY`, `USE_TLS`, `CERT_DIR`.
- gRPC endpoint split is important: `PV_IDENT_URL` is the identity service and `PV_PROC_URL` is the processor service.
- TPass config fails fast when missing `EFR_REMOTE_URL`, `EFR_REMOTE_USER`, or `EFR_REMOTE_PWD`.
- Repo and live tests may also require local DB or Paravision-specific env vars.

## Code Style Guidelines

Follow the local style in the touched file first. The codebase is somewhat mixed, so prefer small, consistent edits over cleanup churn.

### Imports and Modules

- Keep `mod` / `pub mod` declarations at the top.
- This workspace often uses `#[macro_use] mod macros;` first in crate roots; preserve that pattern.
- Group imports roughly by local crate, third-party crates, and std when editing nearby code.
- Do not reorder imports just for style if the file already has a stable pattern.
- Remove unused imports in touched code unless they are about to be used.

### Formatting

- Use `cargo fmt --all`; the workspace has `rustfmt.toml`.
- Current rustfmt preferences include `max_width = 100`, `chain_width = 80`, `fn_call_width = 80`, and some small-item single-line formatting.
- Keep SQL strings and JSON payloads readable; multiline raw strings are preferred for larger SQL.
- Avoid collapsing non-trivial logic into dense one-liners.

### Types and Data Modeling

- Prefer typed structs/enums over ad hoc `serde_json::Value` when the payload shape is known and reused.
- Keep existing result aliases such as `FRResult<T>`, `RepoResult<T>`, and local aliases like `WResult<T>`.
- Use `Option<T>` only when absence is meaningful in the API or domain model.
- Preserve serde attributes already used for compatibility, such as aliases and `skip_serializing_if`.
- When adding conversion layers, prefer `From` implementations over hand-written transformation code at every callsite.

### Naming

- Use `snake_case` for functions, modules, variables, and fields.
- Use `UpperCamelCase` for structs, enums, and traits.
- Use `SCREAMING_SNAKE_CASE` for constants and env-var identifiers.
- Keep domain names explicit: `EnrollmentDeleteResult`, `PossibleMatch`, `CreateIdentitiesRequest`, etc.

### Async and Concurrency

- Use async/await consistently with Tokio.
- Avoid blocking operations in async request paths.
- Shared services are typically wrapped in `Arc`; follow that established pattern.
- When fan-out or post-processing gets complex, introduce helper functions or local maps instead of deeply nested loops.

### Error Handling

- Prefer typed errors like `FRError`, `TPassError`, `RepoError`, and `AppError`.
- Use `?` and `From` conversions where possible instead of repetitive `map_err` boilerplate.
- Avoid new `unwrap()` / `expect()` in runtime paths; tests may use them freely.
- Preserve the existing JSON error envelope shape: `code`, `message`, `details`.
- In `fr-api`, note the existing convention that many domain/API failures are returned as HTTP 200 with a structured error body; do not change that behavior unless requested.

### Logging and Observability

- Use `tracing` macros for service and API code.
- Include useful identifiers like `fr_id`, `ext_id`, operation name, or endpoint context.
- Do not log secrets, credentials, tokens, or full sensitive payloads.
- Avoid adding noisy debug logs to hot paths unless they are clearly justified.

### Database and SQLx

- Keep SQL parameterized with `.bind(...)`.
- Prefer `query_as` into typed records when shape is known.
- Return SQLx failures through the repo/domain error types; do not swallow write failures.
- Preserve current table and schema naming; do not “clean up” SQL naming unless the task requires it.

### API / Serialization Patterns

- Preserve existing request/response shapes, especially V1 compatibility endpoints under `/fr`.
- Keep serde renames and aliases that support backward compatibility.
- Multipart extractors and JSON handlers should validate inputs early and return existing error types.
- When changing API behavior, document the affected route and payload shape.

### Testing Conventions

- Prefer small unit tests near the code when logic is isolated.
- Keep live or environment-dependent coverage in integration tests and mark them ignored when they require external systems.
- If adding a regression test, make the name describe the behavior, not the implementation.

### Cleanup Policy

- This repo contains legacy comments, TODOs, and some inconsistent naming.
- Do not perform broad renames or style rewrites unless requested.
- Improve touched areas incrementally, without changing behavior accidentally.

## Agent Checklist

- Run `cargo fmt --all` after Rust edits.
- Run `cargo check --workspace` or at least the affected crate.
- Run the most relevant test command you can without relying on unavailable services.
- Call out any skipped live tests or env-dependent verification.
- If API behavior or env assumptions changed, mention that in your handoff.
