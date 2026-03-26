# `fr-api` Reference

This document is the public-facing reference for the current `/fr/v2` API.

## Conventions Used Here

- multipart examples use `curl -F`
- JSON examples use `curl -H "Content-Type: application/json" -d ...`
- sample values are illustrative and should be adjusted for your environment
- business errors may still be returned with HTTP `200`

## Enrollment

### `POST /fr/v2/enrollment/create`

Create a new FR enrollment from an uploaded image and a `details` payload.

Content type:

- `multipart/form-data`

Multipart fields:

- `image` (required) - JPEG or PNG image data
- `details` (required) - JSON object deserialized into `EnrollDetails`

Supported `details` forms:

Minimal details:

```json
{
  "kind": "Min",
  "first_name": "Ryan",
  "last_name": "Martin",
  "ext_id": "1001"
}
```

TPass-backed details:

```json
{
  "kind": "TPass",
  "ccode": 1001,
  "fName": "Ryan",
  "lName": "Martin"
}
```

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/create" \
  -F 'details={"kind":"Min","first_name":"Ryan","last_name":"Martin","ext_id":"1001"};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Success response:

```json
{
  "fr_id": "i_12345",
  "ext_id": "1001"
}
```

Example business error:

```json
{
  "code": "DUPLICATE_ERR",
  "message": "an enrollment already exists that matches face",
  "details": {
    "ext_id": "1001",
    "fr_id": "i_98765",
    "score": 0.9931
  }
}
```

Example input error:

```json
{
  "code": "GENERIC_ERR",
  "message": "Missing enrollment details",
  "details": null
}
```

Notes:

- `ext_id` is required in practice for successful enrollment
- image quality and duplicate checks happen before identity creation

### `POST /fr/v2/enrollment/search`

Search locally stored enrollments by last name.

Content type:

- `application/json`

Input:

```json
{
  "last_name": "Martin"
}
```

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/search" \
  -H "Content-Type: application/json" \
  -d '{"last_name":"Martin"}'
```

Success response:

```json
[
  {
    "fr_id": "i_12345",
    "ext_id": "1001",
    "first_name": "Ryan",
    "last_name": "Martin",
    "middle_name": null,
    "img_url": null
  }
]
```

Example business error:

```json
{
  "code": "REPO_ERR",
  "message": "database error message here",
  "details": null
}
```

Notes:

- the current implementation only supports search-by-last-name
- empty search text returns an empty array at the service layer

### `POST /fr/v2/enrollment/delete`

Delete an enrollment by FR identity id.

Content type:

- `application/json`

Input:

```json
{
  "fr_id": "i_12345"
}
```

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/delete" \
  -H "Content-Type: application/json" \
  -d '{"fr_id":"i_12345"}'
```

Success response:

```json
{
  "fr_id": "i_12345"
}
```

Example business error:

```json
{
  "code": "INVALID_INPUT",
  "message": "you must delete by fr_id",
  "details": null
}
```

Notes:

- although the payload enum still contains legacy shapes, the current validation path only accepts `fr_id`

### `POST /fr/v2/enrollment/add-face`

Add a secondary face to an existing identity.

Content type:

- `multipart/form-data`

Multipart fields:

- `image` (required)
- `fr_id` (required)

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/add-face" \
  -F "fr_id=i_12345" \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Success response:

```json
{
  "face_id": "f_56789",
  "fr_id": "i_12345",
  "created_at": "2026-03-22T18:30:00Z",
  "quality": 0.98
}
```

Example business error:

```json
{
  "code": "ADD_FACE_ERR",
  "message": "could not add face for identity",
  "details": {
    "fr_id": "i_12345"
  }
}
```

Example extractor error:

- invalid or missing multipart image payload may be returned as `400` or as a `GENERIC_ERR` envelope,
  depending on where the failure occurs

### `POST /fr/v2/enrollment/delete-faces`

Delete one or more stored face records from an identity.

Content type:

- `application/json`

Input:

```json
{
  "fr_id": "i_12345",
  "face_ids": ["f_56789", "f_56790"]
}
```

Success response:

```json
{
  "rows_affected": 2,
  "fr_id": "i_12345",
  "face_ids": ["f_56789", "f_56790"]
}
```

Example business error:

```json
{
  "code": "GENERIC_ERR",
  "message": "fr_id and at least one face_id are required",
  "details": null
}
```

### `POST /fr/v2/enrollment/get-faces`

List stored faces for an identity.

Content type:

- `application/json`

Input:

```json
{
  "fr_id": "i_12345"
}
```

Success response:

```json
[
  {
    "face_id": "f_56789",
    "fr_id": "i_12345",
    "created_at": "2026-03-22T18:30:00Z",
    "quality": 0.98
  }
]
```

Example business error:

```json
{
  "code": "GENERIC_ERR",
  "message": "fr_id was empty. Did you send one?",
  "details": null
}
```

### `GET /fr/v2/enrollment/metadata`

Return aggregate counts for local enrollment state.

Example request:

```bash
curl "http://localhost:3000/fr/v2/enrollment/metadata"
```

Success response:

```json
{
  "profiles_total": 10,
  "profiles_with_fr_id": 10,
  "images_total": 10,
  "registration_errors_total": 0,
  "enrollment_logs_total": 14
}
```

### `GET /fr/v2/enrollment/roster`

Return a roster-style list of locally stored enrollments.

Success response:

```json
[
  {
    "fr_id": "i_12345",
    "ext_id": "1001",
    "first_name": "Ryan",
    "last_name": "Martin",
    "middle_name": null,
    "img_url": null
  }
]
```

Notes:

- current implementation is capped at `1000` records and does not page

### `POST /fr/v2/enrollment/errlog`

Return recent enrollment error-log records.

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/errlog"
```

Success response:

```json
[
  {
    "id": 1,
    "code": "create_enrollment",
    "error": {},
    "input": {},
    "created_at": "2026-03-22T18:30:00Z",
    "updated_at": "2026-03-22T18:30:00Z"
  }
]
```

### `POST /fr/v2/enrollment/reset`

Delete all local enrollment state.

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/reset"
```

Success response:

```json
{
  "msg": "All enrollments deleted",
  "total": 42
}
```

Notes:

- this is a destructive operator action and should not be used in routine client flows

## Recognition and Validation

### `POST /fr/v2/recognize`

Recognize faces in an uploaded image.

Content type:

- `multipart/form-data`

Multipart fields:

- `image` (required)
- `opts` (optional JSON)

Supported `opts` fields for this route:

- `top_matches` - maximum number of matches requested per face
- `include_details` - include remote profile details in `possible_matches[].details`

Additional `opts` fields may be accepted but are not used by this route.

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/recognize" \
  -F 'opts={"include_details":true,"top_matches":1};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Success response:

```json
[
  {
    "face": {
      "bbox": {
        "origin": { "x": 120.0, "y": 45.0 },
        "width": 150.0,
        "height": 150.0
      },
      "acceptability": 0.99,
      "quality": 0.98,
      "mask": null,
      "liveness": null,
      "template": null
    },
    "possible_matches": [
      {
        "fr_id": "i_12345",
        "score": 0.9923,
        "score_pct": 99.23,
        "ext_id": "1001",
        "details": {
          "ccode": 1001,
          "fName": "Ryan",
          "lName": "Martin"
        }
      }
    ]
  }
]
```

Example business error:

```json
{
  "code": "GENERIC_ERR",
  "message": "An image is required but was not provided",
  "details": null
}
```

### `POST /fr/v2/detect-faces`

Detect faces without running identity recognition.

Input:

- multipart `image` (required)

Success response:

```json
[
  {
    "bbox": {
      "origin": { "x": 120.0, "y": 45.0 },
      "width": 150.0,
      "height": 150.0
    },
    "acceptability": 0.99,
    "quality": 0.98,
    "mask": null,
    "liveness": null,
    "template": null
  }
]
```

### `POST /fr/v2/quality-check`

Return quality and acceptability metrics for the most prominent face in the image.

Input:

- multipart `image` (required)

Success response:

```json
{
  "high_quality": true,
  "image": {
    "min_acceptability": 0.8,
    "min_quality": 0.9,
    "acceptability": 0.96,
    "quality": 0.97
  }
}
```

Example business error:

```json
{
  "code": "FACE_NOT_FOUND_ERR",
  "message": "No faces were detected in image",
  "details": null
}
```

### `POST /fr/v2/liveness-check`

Return combined quality, acceptability, face-location, and liveness data.

Input:

- multipart `image` (required)

Success response:

```json
{
  "image": {
    "min_acceptability": 0.8,
    "min_quality": 0.9,
    "acceptability": 0.96,
    "quality": 0.97
  },
  "face": {
    "bounding_box": {
      "origin": { "x": 120.0, "y": 45.0 },
      "width": 150.0,
      "height": 150.0
    }
  },
  "liveness": {
    "min_score": 0.5,
    "score": 0.98,
    "feedback": [],
    "is_live": true
  },
  "is_valid": true
}
```

### `POST /fr/v2/validate-image`

Legacy alias for the liveness-style validation route.

Notes:

- for new integrations, prefer `/fr/v2/liveness-check`

## Attendance

### `POST /fr/v2/mark-attendance`

Recognize a face and optionally record attendance in TPass.

Content type:

- `multipart/form-data`

Multipart fields:

- `image` (required)
- `opts` (recommended)

Recommended `opts` shape:

```json
{
  "top_matches": 1,
  "include_detected_faces": false,
  "on_match": "check_out",
  "min_match": 0.95,
  "rec_location": "Front Gate",
  "include_details": true
}
```

Important fields:

- `top_matches` - number of candidates to request
- `include_details` - should usually be `true` for attendance
- `on_match` - `check_in`, `check_out`, or any other string for recognition-only logging
- `rec_location` - written to the local recognition log

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/mark-attendance" \
  -F 'opts={"top_matches":1,"include_details":true,"on_match":"check_out","rec_location":"Front Gate"};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Success response:

```json
{
  "identity": {
    "face": {
      "bbox": {
        "origin": { "x": 120.0, "y": 45.0 },
        "width": 150.0,
        "height": 150.0
      },
      "acceptability": 0.99,
      "quality": 0.98,
      "mask": null,
      "liveness": null,
      "template": null
    },
    "possible_matches": [
      {
        "fr_id": "i_12345",
        "score": 0.9923,
        "score_pct": 99.23,
        "ext_id": "1001",
        "details": {
          "ccode": 1001,
          "idnumber": "1001",
          "fName": "Ryan",
          "lName": "Martin"
        }
      }
    ]
  },
  "status": {
    "time_stamp": "2026-03-22T18:30:00",
    "tardy": false,
    "kind": "Out"
  }
}
```

Example business error:

```json
{
  "code": "GENERIC_ERR",
  "message": "recognized face has no saved details. Can't mark for attendance",
  "details": null
}
```

Notes:

- if recognition succeeds but no remote details are attached, attendance can still fail
- `include_details=true` is strongly recommended

## Profile and Alert Routes

### `POST /fr/v2/create-profile`

Create a TPass profile and then enroll the person into the FR backend.

Content type:

- `multipart/form-data`

Multipart fields:

- `image` (required)
- `profile` (required JSON matching `NewProfileRequest`)

Example `profile` JSON:

```json
{
  "compId": 1,
  "clntTid": 1,
  "sttsId": 1,
  "fName": "Ryan",
  "lName": "Martin",
  "type": "Visitor",
  "street1": "1 Main St",
  "city": "Austin",
  "state": "TX",
  "zipcode": "78701"
}
```

Success response:

```json
{
  "fr_id": "i_12345",
  "ext_id": "1001"
}
```

Example business error:

```json
{
  "code": "GENERIC_ERR",
  "message": "Creating a profile requires personal info which was not provided",
  "details": null
}
```

Notes:

- this route is useful for clients that want a single create-profile-plus-enroll operation

### `POST /fr/v2/edit-profile`

Edit a remote TPass profile.

Content type:

- `application/json`

Input shape:

- JSON body matching `EditProfileRequest`

Example request body:

```json
{
  "compId": 1,
  "ccode": 1001,
  "clntTid": 1,
  "sttsId": 1,
  "fName": "Ryan",
  "lName": "Martin",
  "idnumber": "1001",
  "street1": "1 Main St",
  "state": "TX",
  "zipcode": "78701"
}
```

Success response:

- raw TPass edit response JSON

Notes:

- this route is effectively a remote passthrough and its response shape is less stable than the core enrollment routes

### `POST /fr/v2/send-alert`

Send an FR alert to TPass.

Content type:

- `application/json`

Input:

```json
{
  "CompId": 1,
  "PInfo": 42,
  "Type": "FR Alert",
  "Image": null
}
```

Example request:

```bash
curl -X POST "http://localhost:3000/fr/v2/send-alert" \
  -H "Content-Type: application/json" \
  -d '{"CompId":1,"PInfo":42}'
```

Success response:

```json
{
  "message": "alert sent"
}
```

Example extractor error:

- missing required JSON fields will typically return HTTP `422`

## Legacy Compatibility Summary

The service still mounts legacy routes under `/fr`, including compatibility forms of recognize,
enrollment create/delete, add-face, get-identity, create-profile, edit-profile, and send-alert.

For new clients, use `/fr/v2` exclusively.
