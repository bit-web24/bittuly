# syntax=docker/dockerfile:1
FROM rust:1.96-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY libs/ libs/
COPY services/ services/

RUN cargo build --release --bin consumer-service

# rust:1.96-slim uses debian:trixie-slim as the base image under the hood so,
# the compiled binary is compatible with it.
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash appuser
USER appuser

COPY --from=builder /build/target/release/consumer-service /usr/local/bin/consumer-service

CMD ["consumer-service"]
