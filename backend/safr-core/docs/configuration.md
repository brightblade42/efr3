# Configuration

`fr-api` is configured entirely through environment variables loaded at startup.

## Startup Model

On boot, `fr-api`:

1. loads `.env` with `dotenvy`
2. parses `AppConfig::from_env()`
3. creates a Postgres pool
4. constructs the TPass client
5. constructs the FR backend dispatcher
6. starts the Axum server

If required environment variables are missing or invalid, startup fails early.

## Core Environment Variables

### Remote System

- `EFR_REMOTE_NAME` - remote implementation selector, typically `tpass`
- `EFR_REMOTE_URL` - base URL for the TPass API
- `EFR_REMOTE_USER` - TPass username
- `EFR_REMOTE_PWD` - TPass password

### FR Engine

- `EFR_ENGINE_NAME` - backend selector, typically `paravision` or `pv`
- `EFR_ENGINE_IDENT_ADDR` - Paravision identity host
- `EFR_ENGINE_IDENT_PORT` - Paravision identity port
- `EFR_ENGINE_PROC_ADDR` - Paravision processor host
- `EFR_ENGINE_PROC_PORT` - Paravision processor port

### Postgres

- `EFR_DB_ADDR`
- `EFR_DB_PORT`
- `EFR_DB_USER`
- `EFR_DB_PWD`
- `EFR_DB_NAME`
- `EFR_DB_SSLMODE` - optional, defaults to `disable`
- `EFR_DB_MAX_CONN` - optional, defaults to `10`

### Matching and Quality Thresholds

- `EFR_MIN_MATCH`
- `EFR_MIN_DUPE_MATCH`
- `EFR_MIN_SECONDARY_MATCH`
- `EFR_MAX_MATCHES_PER_FACE`
- `EFR_MIN_QUALITY`
- `EFR_MIN_ACCEPTABILITY`

Threshold values may be supplied either as ratios (`0.95`) or percentages (`95`).

### Server

- `EFR_SERVER_PORT` - optional, defaults to `3000`

## Operational Notes

- the Paravision processor and identity endpoints are separate and both must be configured
- TPass configuration is mandatory for profile, alert, attendance, and enrichment workflows
- local DB access is required for roster, metadata, and logging flows
- some tests require additional environment variables dedicated to live gRPC smoke checks

## TLS and Security Note

`TPassClient` currently builds the reqwest client with invalid-certificate acceptance enabled. That
may be necessary in some environments, but it should be treated as a deployment-time security choice
and reviewed carefully for production.

## Recommended Local Validation

After setting env vars, the most useful checks are:

```bash
cargo check --workspace
cargo run -p fr-api
./examples/vibe-check
```
