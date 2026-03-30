# syntax=docker/dockerfile:1
FROM docker.io/debian:bookworm-slim

# Install SSL certs for your internal API calls
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

#this is the root of the application
WORKDIR /app

# The binary path is relative to the build context (project root)
COPY ../target/x86_64-unknown-linux-gnu/release/fr-api ./fr-api

# Security: Run as non-root
USER 1000:1000

EXPOSE 8080

ENTRYPOINT ["./fr-api"]
