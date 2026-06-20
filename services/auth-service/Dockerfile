# syntax=docker/dockerfile:1

# rust:1.96-slim is Debian bookworm-slim — glibc 2.36, matches the runtime stage
FROM rust:1.96-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY libs/ libs/
COPY services/ services/

RUN cargo build --release --bin auth-service

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash appuser
USER appuser

COPY --from=builder /build/target/release/auth-service /usr/local/bin/auth-service

EXPOSE 3001
CMD ["auth-service"]
