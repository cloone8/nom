# Builder image. Keep rust-toolchain file in sync with Rust version here
FROM rust:1.91.1-alpine AS builder

RUN rustup target add wasm32-unknown-unknown

# Additional build tools needed for/by cargo-leptos
RUN apk add --no-cache alpine-sdk perl npm

RUN npm install -g sass

RUN cargo install --locked cargo-leptos@0.3.1

RUN mkdir -p /app
WORKDIR /app

# Source files
COPY src src
COPY public public
COPY style style
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Build the actual webapp and server
RUN cargo leptos build --release -vv

# Runner image. Just copies the artifacts
FROM alpine:latest AS runner

WORKDIR /app

COPY --from=builder /app/target/release/nom /app/
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/Cargo.toml /app/

# NOTE: DSM doesn't properly take these as defaults if importing this image as a tarball
ENV LEPTOS_ENV="PROD"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT=/app/site
ENV NOM_DB="nom.db"

EXPOSE 8080

ENTRYPOINT ["/app/nom"]
