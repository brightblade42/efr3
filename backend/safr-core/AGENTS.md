# AGENTS.md

Guidance for agentic coding agents working in `safr-core` (Rust workspace).

## Scope

- This file applies to the workspace rooted at `backend/safr-core`.
- Crates in this workspace: `fr-api`, `cv-cli`, `libfr`, `libpv`, `libtpass`.
- Primary stack: Rust 2021, Tokio async runtime, Axum API server, SQLx/Postgres, Reqwest.

## Rule Files (Cursor / Copilot)

- Checked for Cursor rules in `.cursor/rules/` and `.cursorrules` under `/Users/ryan/projects/exp/safr`.
- Checked for Copilot rules in `.github/copilot-instructions.md` under `/Users/ryan/projects/exp/safr`.
- No Cursor or Copilot instruction files were found at the time this AGENTS.md was generated.
- If any of those files are added later, treat them as higher-priority guidance and update this file.

## Workspace Layout

- `fr-api`: Axum HTTP service exposing FR + TPass endpoints.
- `cv-cli`: CLI helper for enrollment and indexing workflows.
- `libfr`: orchestration/domain layer; backend trait + implementations.
- `libpv`: Paravision API client/types/errors.
- `libtpass`: TPass API client/config/types/errors.

## Build / Lint / Test Commands

Run commands from `backend/safr-core` unless noted.

### Build

- Build all crates: `cargo build --workspace`
- Build release: `cargo build --workspace --release`
- Build one crate: `cargo build -p fr-api`
- Check only (faster): `cargo check --workspace`

### Lint / Format

- Format all crates: `cargo fmt --all`
- Check formatting only: `cargo fmt --all --check`
- Run clippy (workspace): `cargo clippy --workspace --all-targets`
- Clippy on one crate: `cargo clippy -p libfr --all-targets`

### Tests

- Run all tests in workspace: `cargo test --workspace`
- Run tests for one crate: `cargo test -p libfr`
- Run doc tests too (if present): `cargo test --workspace --doc`

### Single-Test Recipes (important)

- Single unit test by name in a crate:
  - `cargo test -p libfr test_name_substring`
- Exact single test (recommended when names overlap):
  - `cargo test -p libfr exact_test_name -- --exact --nocapture`
- Single integration test target file (`tests/foo.rs`) in crate:
  - `cargo test -p libfr --test foo`
- Single test inside one integration test target:
  - `cargo test -p libfr --test foo specific_case_name -- --exact --nocapture`
- If you do not know the exact test name first:
  - `cargo test -p libfr -- --list`

Note: this repo currently has little/no committed Rust test coverage, so you may need to add tests before using single-test workflows.

## Run Commands

- Start API service: `cargo run -p fr-api`
- Run CLI: `cargo run -p cv-cli -- --help`
- Example with API URL for CLI:
  - `cargo run -p cv-cli -- --url http://localhost:3000 reset`

## Runtime Environment Notes

- `fr-api` reads many env vars; important ones include:
  - Backend is Paravision-only (`FR_BACKEND` and `CV_URL` are removed)
  - `FRAPI_PORT`, `PV_IDENT_URL`, `PV_PROC_URL`
  - `SAFR_DB_ADDR`, `SAFR_DB_PORT`
  - `MIN_MATCH`, `MIN_DUPE_MATCH`, `MIN_QUALITY`, `USE_TLS`, `CERT_DIR`
- `libtpass` config requires env vars and will fail fast if missing:
  - `TPASS_URL`, `TPASS_USER`, `TPASS_PWD`

## Code Style Guidelines

When editing, follow existing local conventions in each file first; apply these standards for new/changed code.

### Imports and Module Organization

- Keep module declarations at top (`mod ...`, `pub mod ...`) before other imports.
- Prefer grouped `use` statements by origin (std, external crates, local crate).
- Preserve existing local ordering style; do not churn imports unnecessarily.
- Remove unused imports when touching a file unless intentionally kept for near-term work.

### Formatting

- Use `rustfmt` defaults (`cargo fmt --all`) as baseline formatting authority.
- Keep lines readable; avoid dense single-line blocks unless idiomatic.
- Preserve current brace and match-arm style used in the surrounding file.

### Types and Data Modeling

- Prefer explicit structs/enums over raw `serde_json::Value` when practical.
- Use `Option<T>` only when absence is meaningful; otherwise require fields.
- Keep domain error/result aliases (`type XResult<T> = Result<T, XError>`) for readability.
- Maintain `#[derive(Serialize, Deserialize, Debug, Clone)]` patterns where serialization is required.

### Naming Conventions

- `snake_case` for functions, variables, and modules.
- `UpperCamelCase` for structs/enums/traits.
- `SCREAMING_SNAKE_CASE` for constants.
- For API payload types, keep names descriptive and domain-specific (`CreateIdentitiesRequest`, etc.).

### Async and Concurrency

- Use async/await with `tokio` consistently; avoid blocking calls in async paths.
- Prefer explicit type aliases for complex join results when concurrency fans out.
- When spawning tasks, ensure returned errors are surfaced/logged meaningfully.

### Error Handling

- Prefer returning typed errors (`FRError`, `TPassError`, `AppError`) over stringly errors.
- Use `?` with `From` conversions where available; add conversions rather than repetitive mapping.
- Avoid introducing new `unwrap()`/`expect()` in request/runtime paths unless failure is truly unrecoverable.
- If a module already uses structured error payloads (`code`, `message`, `details`), keep that shape.
- In HTTP handlers, return consistent API error JSON envelopes used by `AppError`.

### Logging and Observability

- Use `tracing` macros (`debug!`, `info!`, `warn!`, `error!`) rather than `println!` for service/library code.
- Include context in log messages (ids, operation names), but avoid leaking secrets/tokens.
- Keep noisy debug output out of hot paths unless gated by log level.

### Database and SQLx

- Keep SQL statements clear and parameterized (already using `.bind(...)`).
- Propagate SQLx errors via typed error conversions (`From<sqlx::Error> for FRError`).
- Do not silently swallow DB failures in critical write paths; at minimum log with context.

### API / Serialization Patterns

- Keep serde field names compatible with upstream APIs where required.
- Use conversion impls (`From`/`Into`) for request/response transformations between layers.
- Preserve current V1 back-compat endpoints and payload shapes unless explicitly changing API behavior.

### Incremental Cleanup Policy

- This codebase includes legacy TODOs, comments, and some inconsistent style.
- Do not perform broad stylistic rewrites unless requested.
- Make focused, minimal, safe edits around the task.
- If you touch a messy area, improve it incrementally without changing semantics.

## Agent Change Checklist

- Run: `cargo fmt --all`
- Run: `cargo check --workspace` (or crate-scoped check for faster iteration)
- Run relevant tests (`cargo test -p <crate>` or specific test command)
- If API behavior changed, mention affected endpoints and payload shape changes.
- If env/config assumptions changed, document required vars in PR/commit notes.

## Known Practical Notes for Agents

- Workspace has multiple crates but no top-level Makefile/task runner; use Cargo directly.
- Some modules are intentionally incomplete (`todo!()` paths exist); avoid invoking unfinished paths in tests.
- API code includes V1 compatibility routes; preserve them unless migration work is requested.
