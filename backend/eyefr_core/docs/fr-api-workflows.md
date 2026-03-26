# `fr-api` Workflows

This document shows common end-to-end usage patterns for API consumers.

## Create a Basic Enrollment

Use this flow when you already know the person's external id and do not need to create a remote
profile in the same request.

### Step 1: Create the enrollment

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/create" \
  -F 'details={"kind":"Min","first_name":"Ryan","last_name":"Martin","ext_id":"1001"};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Example response:

```json
{
  "fr_id": "i_12345",
  "ext_id": "1001"
}
```

### Step 2: Inspect stored faces

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/get-faces" \
  -H "Content-Type: application/json" \
  -d '{"fr_id":"i_12345"}'
```

### Step 3: Add another face if needed

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/add-face" \
  -F "fr_id=i_12345" \
  -F "image=@/path/to/face-2.jpg;type=image/jpeg"
```

## Recognize a Face

Use this flow when you only need identity lookup.

```bash
curl -X POST "http://localhost:3000/fr/v2/recognize" \
  -F 'opts={"top_matches":1,"include_details":true};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Use `include_details=true` if your client wants enriched remote profile data.

## Mark Attendance

Use this flow when you want recognition plus a TPass attendance side effect.

```bash
curl -X POST "http://localhost:3000/fr/v2/mark-attendance" \
  -F 'opts={"top_matches":1,"include_details":true,"on_match":"check_in","rec_location":"Front Gate"};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

Recommendations:

- set `include_details=true`
- set `top_matches` to `1` unless your client has a reason to inspect more candidates
- set `rec_location` so logs are useful later

## Create a Remote Profile and Enroll in One Call

Use this flow when you want SAFR to create the person in TPass and enroll them in FR in one action.

```bash
curl -X POST "http://localhost:3000/fr/v2/create-profile" \
  -F 'profile={"compId":1,"clntTid":1,"sttsId":1,"fName":"Ryan","lName":"Martin"};type=application/json' \
  -F "image=@/path/to/face.jpg;type=image/jpeg"
```

The response is the same `IDPair` returned by standard enrollment.

## Delete an Enrollment

```bash
curl -X POST "http://localhost:3000/fr/v2/enrollment/delete" \
  -H "Content-Type: application/json" \
  -d '{"fr_id":"i_12345"}'
```

If you only need to remove extra face records and keep the identity, use `delete-faces` instead.

## Troubleshooting

### I got HTTP 200 but the request still failed

Inspect the response body. Many business failures are returned as JSON error envelopes rather than as
non-200 transport errors.

### Attendance recognition worked, but attendance still failed

This usually means the top match did not include the remote identifiers required for TPass attendance.
Make sure the request used `include_details=true` and that the remote profile contains `idnumber` and
`ccode`.

### My image upload failed

Make sure the `image` field contains a real JPEG or PNG payload. The extractor validates image bytes
and rejects unrecognized formats.
