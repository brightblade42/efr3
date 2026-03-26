# `libfr` Engineering Guide

`libfr` is the internal orchestration crate for SAFR. This guide is aimed at engineers working on
backend behavior rather than external API consumers.

## Design Goals

- keep HTTP concerns out of the core workflow logic
- isolate engine-specific and remote-specific integrations behind traits
- preserve typed models for stable internal contracts
- centralize business workflows in one service layer
- keep local persistence scoped to support data, not FR identity ownership

## Main Entry Points

### `service::FRService`

`FRService` is the main workflow coordinator. If you are tracing behavior across the system, start
here.

Important methods:

- `create_enrollment`
- `delete_enrollment`
- `recognize`
- `detect_faces`
- `add_face`
- `delete_faces`
- `get_faces`
- `get_enrollment_metadata`
- `get_enrollments_by_last_name`
- `log_cam_fr_match`

### `dispatch::FRBackend`

This trait describes the engine-facing operations that the service layer expects. The active
implementation is `PVBackend`.

Use this seam when:

- introducing a new FR engine
- isolating engine-specific bugs
- comparing backend behavior across implementations

### `dispatch::AssetStore`

This trait describes the remote system operations that the service layer expects. The active
implementation is `AssetDispatcher` wrapping `TPassClient`.

Use this seam when:

- integrating a new remote system of record
- changing profile-enrichment behavior
- adjusting attendance or profile-registration workflows

## Core Types

### Enrollment

- `EnrollData` - normalized enrollment input containing image bytes and details
- `EnrollDetails` - either minimal local details or a full TPass profile
- `IDPair` - result that links `fr_id` to `ext_id`

### Recognition

- `MatchConfig` - thresholds and output controls for recognition and duplicate checks
- `FRIdentity` - one detected face plus ranked candidate matches
- `PossibleMatch` - one candidate match including score, external id, and optional details
- `Face` - face-level quality, bbox, liveness, and template data

### Search and Remote Data

- `SearchBy` - remote lookup strategy
- `SearchResult` - remote lookup output
- `RemoteDetails` - typed remote profile payload attached to matches

## How the Layers Interact

### Enrollment

- service validates input
- backend checks duplicates and quality
- repo stores a local profile snapshot
- backend creates identity in Paravision
- remote system registers the identity

### Recognition

- backend returns lookup matches
- service optionally resolves external details by id
- service logs matches locally

### Attendance

- service relies on recognition results enriched with remote details
- TPass-specific identifiers are extracted from the top match
- local DB records the camera recognition event

## Repository Boundaries

`SqlxFrRepository` does not own biometric truth. It owns local support state only. Use the repo for:

- profile snapshots
- images cached locally for enrollment support
- roster and metadata queries
- enrollment and match logs

Do not use it to replace Paravision identity operations.

## Error Model

The dominant domain error type is `FRError`. It absorbs:

- repository failures
- Paravision API failures
- TPass failures
- domain-specific workflow failures such as duplicate detection or poor quality

When adding behavior, prefer extending typed errors or `From` conversions instead of returning plain
strings.

## Common Engineering Tasks

### Add a New FR Backend

1. implement `FRBackend`
2. extend `FRDispatcher::new`
3. keep output mapped into existing `libfr::types`
4. avoid leaking backend-native payloads outside the backend module

### Add a New Remote System

1. implement `AssetStore`
2. extend `AssetDispatcher::new`
3. define or adapt the remote profile model behind `RemoteDetails`
4. keep `FRService` workflow code as generic as possible

### Add a New API Workflow

1. define or reuse stable request/response types in `libfr`
2. implement orchestration in `FRService`
3. only then add HTTP extraction and handler code in `fr-api`

## Testing Notes

- `libfr/src/lib.rs` contains small unit tests for utility and serialization behavior
- `libfr/tests/` contains live or integration-style tests for Paravision and repo behavior
- many tests are environment-dependent and some are intentionally ignored by default

Prefer exact single-test execution when touching live integrations.
