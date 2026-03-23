# Overview

`safr-core` is the Rust backend workspace that powers facial-recognition enrollment, recognition,
attendance, and profile-management workflows.

At a high level, the system combines three major responsibilities:

- face processing and identity lookup through Paravision
- person profile, attendance, and alerting operations through TPass
- local Postgres persistence for profile snapshots, metadata, match logs, and enrollment error logs

## Workspace Crates

### `fr-api`

`fr-api` is the HTTP service built with Axum. It exposes:

- the current public API under `/fr/v2`
- legacy compatibility routes under `/fr`
- internal helper routes under `/tpass`

The service loads environment configuration, builds the shared runtime, parses multipart and JSON
requests, and translates domain failures into the JSON error model used by clients.

### `libfr`

`libfr` is the core orchestration and integration crate. It provides:

- service-level workflow coordination through `FRService`
- backend abstraction through `FRBackend` and `FRDispatcher`
- remote-system abstraction through `AssetStore` and `AssetDispatcher`
- repository access through `SqlxFrRepository`
- shared domain types such as `EnrollData`, `MatchConfig`, `FRIdentity`, and `PossibleMatch`

## External Systems

### Paravision

Paravision is the active facial-recognition backend. The workspace talks to two gRPC services:

- processor service for face detection, embeddings, quality, and liveness-related data
- identity service for identity creation, lookup, add-face, delete-face, and face listing

### TPass

TPass is the remote system of record for people and attendance. It is used for:

- client profile lookup and creation
- profile edits
- attendance marking
- alert delivery
- resolving external details for recognition results

### Postgres

Postgres stores supporting local state rather than acting as the FR engine. The local schema is used
for items such as:

- profile snapshots in `eyefr.profiles`
- stored images in `eyefr.images`
- registration and enrollment error logs
- camera recognition logs in `logs.matches`
- aggregate metadata for admin-style views

## Core Capabilities

- create an FR enrollment from an uploaded image plus minimal details or a TPass profile
- search local enrollments by last name
- retrieve enrollment metadata and roster snapshots
- add and delete secondary faces for an identity
- detect faces, run quality checks, run liveness checks, and recognize identities
- enrich recognition matches with remote profile details
- mark attendance based on a recognition event
- create and edit remote profiles as part of enrollment workflows

## Important API Behavior

The API intentionally mixes two error styles:

- request parsing and body-shape failures are returned as transport errors such as HTTP `400` or `422`
- many business and domain failures are returned as HTTP `200` with a JSON error envelope containing
  `code`, `message`, and `details`

This is a deliberate compatibility choice in the current service and should be treated as part of the
API contract.
