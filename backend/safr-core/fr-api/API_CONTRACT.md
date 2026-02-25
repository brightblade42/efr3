# FR API Contract (V2)

This document captures the current request/response contract for migrated `/fr/v2` endpoints.

## Scope

- Active API namespace: `/fr/v2`
- Legacy `/fr` (v1) routes are removed from this service.

## Identifier Rules

- `ccode` is a TPass identifier and is always numeric (`u64`).
- `ext_id` is an external identifier and is always treated as string in FR internals.
  - API responses may include both:
    - `ext_id` (numeric compatibility field)
    - `ext_id_str` (canonical string field)

## Error Behavior

There are two error classes:

1. **Application errors** (`AppError`) return HTTP `200` with a standard JSON envelope:

```json
{
  "code": 1061,
  "message": "Failed to search enrollments from eyefr repository",
  "details": {
    "term": "User",
    "error": "..."
  }
}
```

2. **Extraction/validation errors from Axum** (missing required query/body fields) return HTTP `400` or `422`.

## Endpoint Contracts

### `POST /fr/v2/enrollment/add-face`

- Query params:
  - `fr_id` (required, non-empty)
- Content type:
  - `multipart/form-data`
- Multipart fields:
  - `image` (required)
  - `opts` (optional JSON)
- Success response:
  - JSON object including `faces` array (face records from backend)
- Contract checks:
  - Missing `fr_id` => HTTP `400`

### `POST /fr/v2/enrollment/delete-face`

- JSON body:

```json
{
  "fr_id": "<fr-id>",
  "face_id": "<face-id>"
}
```

- Success response:
  - JSON object from backend delete-face operation
  - includes `rows_affected` in current backend responses
- Contract checks:
  - Missing required fields => HTTP `422`

### `POST /fr/v2/get-identity`

- JSON body:

```json
{
  "fr_id": "<fr-id>"
}
```

- Success response:
  - JSON object with face info payload (`faces`, `next_page_token`, `total_size`)
- Contract checks:
  - Missing required `fr_id` => HTTP `422`

### `POST /fr/v2/recognize`

- Content type:
  - `multipart/form-data`
- Multipart fields:
  - `image` (required)
  - `opts` (optional JSON mapped to `ImageOpts`, including `top_matches`)
- Success response:
  - JSON array of recognition results (`FRIdentity[]`)
  - each result includes `possible_matches[]` entries with:
    - `fr_id`
    - `ext_id`
    - `score` (raw match score in `[0,1]`)
    - `score_pct` (friendly percentage score)
    - `details` (nullable)
- Field semantics:
  - `possible_matches[].score` is used internally for threshold comparison.
  - `possible_matches[].score_pct` is for display/UI friendliness.
  - `possible_matches[].confidence` is not emitted by `/fr/v2/recognize` responses.
- Contract checks:
  - Missing `image` => HTTP `200` + standard error envelope

### `POST /fr/v2/send-alert`

- JSON body (TPass FR alert):

```json
{
  "CompId": 1,
  "PInfo": 42,
  "Type": "FR Alert",
  "Image": "<optional-base64>"
}
```

- Success response:

```json
{
  "message": "alert sent"
}
```

- Contract checks:
  - Missing required payload fields => HTTP `422`

### `POST /fr/v2/enrollment/create`

- Content type:
  - `multipart/form-data`
- Multipart fields:
  - `image` (required)
  - `details` (required JSON object mapped to `EnrollDetails`)
    - for `{"kind":"Min", ...}` payloads, `ext_id` is required
    - for `{"kind":"TPass", ...}` payloads, `ccode` (or `ext_id`) is required
- Success response:
  - JSON object including:
    - `fr_id`
    - `ext_id` (numeric compatibility)
    - `ext_id_str` (canonical string)
- Contract checks:
  - Missing `details` with image present => HTTP `200` + standard error envelope (`code=0`)
  - Missing external id in `details` => HTTP `200` + standard error envelope (`code=1050`)

### `POST /fr/v2/enrollment/delete`

- Accepted JSON body variants:

```json
{"fr_id":"<fr-id>"}
```

```json
{"ccode":123}
```

```json
{"Name":["First","Last"]}
```

```json
{"FullName":{"first":"First","middle":"M","last":"Last"}}
```

- Success response:
  - JSON object with deleted FR identity id (e.g. `{ "fr_id": "..." }`)
- Contract checks:
  - Empty/invalid target => HTTP `200` + standard error envelope

### `POST /fr/v2/enrollment/search`

- JSON body:

```json
{"last_name":"Smith"}
```

- Success response:
  - JSON array of enrollment records
- Contract checks:
  - DB failure path returns HTTP `200` + standard error envelope (`code=1061`)

## Contract Tests

Route contract tests live in `fr-api/src/main.rs` under `mod tests` and currently cover:

- extractor/required-field behavior (`400`/`422`)
- happy-path response shape for moved routes
- standard error envelope behavior on selected enrollment failure paths
