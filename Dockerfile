# Builder image. Keep rust-toolchain file in sync with Rust version here
FROM rust:1.99.1-trixie AS builder

RUN rustup target add wasm32-unknown-unknown

# Downloads dart-sass itself when building
RUN cargo install --locked cargo-leptos@0.3.11

WORKDIR /app

# Source files
COPY src src
COPY style style
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Content hashes in the JS/WASM/CSS file names, so browsers can cache them forever. The server
# picks this up when it's compiled (see main.rs), so the runner needs no such variable.
ENV LEPTOS_HASH_FILES=true

# Build the actual webapp and server
RUN cargo leptos build --release --precompress --lib-cargo-args="--locked" --bin-cargo-args="--locked" -vv

# Runner image. Just copies the artifacts
FROM debian:trixie-slim AS runner

WORKDIR /app

COPY --from=builder /app/target/release/nom /app/
# The hashed file names; the server reads this from next to its binary
COPY --from=builder /app/target/release/hash.txt /app/
COPY --from=builder /app/target/site /app/site

# NOTE: DSM doesn't properly take these as defaults if importing this image as a tarball
ENV LEPTOS_ENV="PROD"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT=/app/site

EXPOSE 8080

ENTRYPOINT ["/app/nom"]
