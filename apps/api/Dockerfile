# Build context is the repository root; see docker-compose.yml.
#
# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

# The `webp` crate compiles vendored libwebp, so the builder needs a C toolchain.
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential pkg-config \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Dependencies first, as their own layer: they change far less often than the
# source, and compiling them is most of the build.
COPY apps/api/Cargo.toml apps/api/Cargo.lock ./
RUN mkdir src \
 && echo 'fn main() {}' > src/main.rs \
 && touch src/lib.rs \
 && cargo build --release \
 && rm -rf src

COPY apps/api/src ./src
# Embedded into the binary by sqlx::migrate! at compile time.
COPY apps/api/migrations ./migrations

# Cargo keys off mtime; the dummy sources above are newer than the real ones.
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --create-home --home-dir /app app

COPY --from=builder /app/target/release/grid-reveal-api /usr/local/bin/grid-reveal-api

WORKDIR /app
# Picture storage when no R2 bucket is configured. A named volume in compose.
RUN mkdir -p /app/data && chown -R app:app /app

USER app
EXPOSE 8080

CMD ["grid-reveal-api"]
