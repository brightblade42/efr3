# Architecture

This document describes the runtime architecture of the SAFR Rust backend.

## Layered View

The system is organized into a small number of explicit layers:

```text
HTTP Client
  -> fr-api handlers and extractors
  -> libfr::service::FRService
     -> libfr::dispatch::FRDispatcher -> Paravision gRPC
     -> libfr::dispatch::AssetDispatcher -> TPass HTTP API
     -> libfr::repo::SqlxFrRepository -> Postgres
```

## `fr-api` Responsibilities

`fr-api` is responsible for:

- loading environment configuration
- creating shared runtime dependencies such as the DB pool and remote clients
- exposing the route tree under `/fr`, `/fr/v2`, and `/tpass`
- parsing multipart payloads and JSON request bodies
- converting `libfr` errors into API responses

`fr-api` does not implement the core FR logic directly. Its job is to normalize HTTP input and hand
off to `libfr`.

## `libfr` Responsibilities

`libfr` owns the application workflows and boundaries between systems.

### Dispatch Layer

The dispatch layer defines the stable seams around concrete integrations.

- `FRBackend` describes the capabilities expected from an FR engine
- `AssetStore` describes the capabilities expected from the remote system of record
- `FRDispatcher` chooses the active FR implementation at startup
- `AssetDispatcher` chooses the active remote implementation at startup

This allows the service layer to depend on stable traits instead of directly on Paravision or TPass.

### Service Layer

`FRService` orchestrates multi-step workflows such as:

- validating enrollment input
- running duplicate and quality checks
- creating a backend identity
- registering the identity with the remote system
- enriching recognition matches with remote details
- writing local logs and metadata

The service layer contains the majority of business behavior.

### Repository Layer

`SqlxFrRepository` provides Postgres-backed access to local support data. It is intentionally scoped
to operational state and reporting rather than FR identity search.

### Paravision Layer

The `pv` module adapts Paravision gRPC calls into internal models used by the service layer. It is
concerned with:

- processing images into face and liveness data
- creating identities and adding faces
- running identity lookups
- retrieving or deleting face records

### TPass Layer

The `tpass` module adapts the TPass HTTP API into internal operations used by SAFR. It is concerned
with:

- token management
- client profile creation and editing
- profile search and enrichment
- attendance status and attendance updates
- alert sending

## Data Ownership

The system intentionally splits ownership across services.

### Paravision Owns

- biometric templates and identity lookup
- stored face records attached to identities
- backend-level quality and liveness data

### TPass Owns

- person profile details and identifiers
- attendance records and related operational state
- alerting operations

### Postgres Owns

- local profile snapshots for SAFR workflows
- enrollment error records
- camera recognition logs
- admin metadata and roster views

## Request Lifecycle

### Recognition Request

1. client uploads an image to `/fr/v2/recognize`
2. `fr-api` extractor validates the multipart payload and image bytes
3. handler builds a `MatchConfig`
4. `FRService::recognize` calls the FR backend
5. Paravision returns candidate matches
6. `FRService` optionally resolves remote details from TPass
7. `fr-api` serializes the final `FRIdentity[]` response

### Enrollment Request

1. client uploads an image and `details` payload to `/fr/v2/enrollment/create`
2. extractor builds `EnrollData`
3. `FRService` validates presence of details and external id
4. `FRService` runs duplicate and quality checks through the FR backend
5. local profile snapshot is written to Postgres
6. backend identity is created in Paravision
7. enrollment is registered with TPass
8. response returns `{ fr_id, ext_id }`

### Attendance Request

1. client uploads an image and attendance-oriented `opts`
2. recognition runs first
3. top match must include remote details such as `idnumber` and `ccode`
4. TPass attendance is updated if `on_match` resolves to check-in or check-out
5. local recognition log is written to Postgres

## Risky and Environment-Sensitive Areas

- startup requires valid engine, DB, and TPass environment variables
- some tests and workflows depend on live Paravision, TPass, and Postgres services
- `enrollment/reset` is destructive and removes local enrollment state
- `mark-attendance` often needs `include_details=true` to succeed because attendance depends on remote identifiers
- TPass client construction currently allows invalid TLS certificates; treat that as a security-sensitive runtime choice
