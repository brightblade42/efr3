# SAFR Core Docs

This directory contains the hand-written engineering and API documentation for the Rust workspace in
`backend/safr-core`.

## Audience

- `libfr` docs are primarily for internal engineers working on SAFR internals.
- `fr-api` docs are mixed-audience docs for internal engineers, operators, and API integrators.

## Document Map

- `overview.md` - project overview, capabilities, and crate responsibilities
- `architecture.md` - system architecture and layer boundaries
- `runtime-flow.md` - request and workflow walkthroughs
- `configuration.md` - environment and runtime configuration
- `libfr.md` - internal engineering guide for the `libfr` crate
- `fr-api.md` - how to use the HTTP API and understand its conventions
- `fr-api-reference.md` - endpoint-by-endpoint reference for the public `/fr/v2` API
- `fr-api-workflows.md` - practical multi-step client workflows

## Generated Code Docs

The workspace also supports generated Rust docs:

```bash
cargo doc --workspace --no-deps
```

For internal-only exploration of private items:

```bash
cargo doc --workspace --no-deps --document-private-items
```

The generated docs are best read alongside the architecture and runtime docs in this folder.
