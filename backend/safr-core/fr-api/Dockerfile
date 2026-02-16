# stage 1 - generate recipe
FROM docker.io/rust as planner
WORKDIR /app
RUN cargo install cargo-chef

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#stage 2 - buid our dependencies
FROM docker.io/rust as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

#stage 3 use the the main official rust docker image as our builder
FROM docker.io/rust as builder
COPY . /app
WORKDIR /app
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --bin fr-api
#FROM gcr.io/distroless/cc-debian11
#FROM debian:stable-slim
FROM docker.io/debian:bookworm-slim
# Install libssl3
RUN apt-get update && apt-get install -y openssl libssl-dev  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fr-api /app/fr-api

RUN mkdir -p /cert
#stage 4 Copy Static Safr-analysis react app
#COPY ./app /app
CMD ["/app/fr-api"]
