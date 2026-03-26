# Runtime Flow

This document walks through the main SAFR workflows as they execute in production.

## Enrollment Flow

Primary endpoint: `POST /fr/v2/enrollment/create`

1. `fr-api` parses multipart form data.
2. The `image` field is normalized to raw image bytes.
3. The `details` field is deserialized into `EnrollDetails`.
4. `FRService::create_enrollment` validates that:
   - details exist
   - an external identifier can be extracted
   - image bytes are present
5. The service runs duplicate and quality checks:
   - `duplicate_check` calls recognition with configured thresholds
   - `ensure_enrollable` requests the most prominent face and validates quality and acceptability
6. A local profile snapshot is written to Postgres.
7. Paravision creates the identity.
8. The new FR identity is registered with the remote system.
9. The API returns an `IDPair`.

Failure modes:

- missing `details` -> API error envelope
- missing external id in the details payload -> API error envelope
- duplicate identity above threshold -> `DUPLICATE_ERR`
- low quality or acceptability -> `QUALITY_LOW_ERR`
- backend or remote errors -> structured API error envelope

## Add Face Flow

Primary endpoint: `POST /fr/v2/enrollment/add-face`

1. `fr-api` extracts `image` and `fr_id` from multipart input.
2. `FRService::add_face` forwards the request to the FR backend.
3. Paravision processes the image, generates an embedding, and attaches the face to the identity.
4. The API returns `EnrolledFaceInfo`.

This flow does not persist additional local profile state.

## Recognition Flow

Primary endpoint: `POST /fr/v2/recognize`

1. `fr-api` extracts the image and optional `opts` payload.
2. `MatchConfig` is derived from app config and then adjusted from request options.
3. `FRService::recognize` calls the FR backend.
4. Paravision returns detected faces and lookup matches.
5. The service optionally loads remote details for the returned external ids.
6. Recognition events are logged.
7. The API returns a list of `FRIdentity` items, sorted left-to-right when multiple faces are present.

## Quality and Liveness Flow

Primary endpoints:

- `POST /fr/v2/quality-check`
- `POST /fr/v2/liveness-check`
- `POST /fr/v2/validate-image` (legacy alias for liveness-style validation)

These routes share the same first stage: extract and validate image bytes. The quality route inspects
the most prominent face and returns threshold comparisons. The liveness route requests additional
backend liveness data and returns a richer object containing image metrics, face bounding box, and
liveness details.

## Attendance Flow

Primary endpoint: `POST /fr/v2/mark-attendance`

1. `fr-api` extracts image bytes and optional recognition settings.
2. `FRService::recognize` runs.
3. The first recognition result is selected.
4. The top possible match must contain remote details.
5. The details payload must contain both `idnumber` and `ccode`.
6. `on_match` is interpreted as:
   - `check_in` -> mark an attendance check-in
   - `check_out` -> mark an attendance check-out
   - any other value -> no TPass attendance call is made
7. The recognition event is logged locally.
8. The API returns the selected identity and the attendance status, if one was produced.

Important note: `include_details=true` is effectively required for a reliable attendance call because
attendance depends on remote profile identifiers.

## Create Profile Flow

Primary endpoint: `POST /fr/v2/create-profile`

1. `fr-api` extracts a multipart `profile` JSON field and an `image` field.
2. The image is base64-encoded and attached to the remote profile request.
3. TPass creates the person profile.
4. TPass profile edit is issued as a follow-up normalization step.
5. The newly created TPass profile is fetched back.
6. The resulting remote profile is wrapped as `EnrollDetails::TPass`.
7. The standard enrollment flow runs.

This endpoint is useful when a client wants to create a person in TPass and create the FR enrollment
in one request.

## Delete and Reset Flow

Primary endpoints:

- `POST /fr/v2/enrollment/delete`
- `POST /fr/v2/enrollment/delete-faces`
- `POST /fr/v2/enrollment/reset`

`delete` removes a single enrollment by FR identity id. `delete-faces` removes selected stored face
records from one identity. `reset` removes all local enrollment state and should be treated as an
operator-only action.
