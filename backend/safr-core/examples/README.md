# API Examples

This folder contains ad-hoc shell examples for a running `fr-api` instance.

These are **not** the official test suite. They are intended for:

- documentation by example
- manual smoke/vibe checks
- quick endpoint shape checks while iterating

## Defaults

- `API_URL` defaults to `http://localhost:3000`
- `FACE_DIR` defaults to `/Users/ryan/faces`
- `DEFAULT_IMAGE` defaults to `${FACE_DIR}/amy1.jpg`

Override per run:

```bash
API_URL="http://localhost:3000" FACE_DIR="/Users/ryan/faces" ./examples/recognize
```

## Quick start

```bash
./examples/vibe-check
```

## Endpoint examples

- `validate-image`
- `detect-faces`
- `recognize`
- `enrollment-create-min`
- `enrollment-search-lastname`
- `enrollment-delete-frid`
- `enrollment-metadata`
- `enrollment-roster`
- `enrollment-errlog`
- `add-face`
- `delete-face`
- `get-identity`
- `mark-attendance`
- `send-alert`

## Notes

- `send-alert` and `mark-attendance` call TPass side-effecting endpoints.
- `enrollment-create-min`, `add-face`, and `enrollment-delete-frid` mutate FR state.
- `matt2.jpg` in `/Users/ryan/faces` is not a valid JPEG payload for processor tests; prefer `amy1.jpg`, `matt1.jpg`, `Ramone1.jpg`, or `normalf.jpeg`.
