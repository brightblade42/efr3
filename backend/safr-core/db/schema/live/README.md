This directory stores live schema snapshots for the identity database.

Files:
- `identity_schema_owned.sql`: app-owned schemas (`eyefr`, `logs`)
- `identity_schema_full.sql`: owned schemas plus dependency schema (`public`)
- `SCHEMA_SOURCE.md`: metadata about the latest capture

Why keep both snapshots:
- `identity_schema_owned.sql` stays focused on the parts we directly own and evolve.
- `identity_schema_full.sql` captures real runtime dependencies on Paravision-managed `public` objects.
- Keeping both avoids mixing ownership concerns while still preserving a complete record.

Refresh requirements:
- PostgreSQL client tools 18+ (`psql`, `pg_dump`), e.g. Homebrew `libpq`.
- `IDENTITY_DB_URL` environment variable set to the target database.

Refresh command:
```bash
./db/schema/live/refresh_identity_schema.sh
```

Example URL format:
```bash
export IDENTITY_DB_URL="postgresql://<user>:<password>@<host>:5432/identity?sslmode=prefer"
```
