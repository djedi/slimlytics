# syntax=docker/dockerfile:1.7
FROM rust:1.98-bookworm AS builder
WORKDIR /src
COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
COPY backend/build.rs ./backend/build.rs
COPY backend/src ./backend/src
COPY docs/openapi.json ./docs/openapi.json
COPY migrations ./migrations
WORKDIR /src/backend
# Preserve dependency builds across source changes on the persistent CI builder.
# Copy the executable outside the mount so it is part of the image layer.
RUN --mount=type=cache,id=slimlytics-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=slimlytics-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=slimlytics-backend-target,target=/src/backend/target,sharing=locked \
    touch src/lib.rs src/main.rs \
    && cargo build --locked --release \
    && cp target/release/slimlytics-backend /usr/local/bin/slimlytics-backend

# Development target for compose.dev.yaml: rebuilds and restarts the API when backend sources,
# migrations, or the OpenAPI document change. Source is bind-mounted at runtime.
FROM rust:1.98-bookworm AS dev
RUN --mount=type=cache,id=slimlytics-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    cargo install --locked cargo-watch
WORKDIR /src/backend
EXPOSE 8080

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --create-home slimlytics
COPY --from=builder /usr/local/bin/slimlytics-backend /usr/local/bin/slimlytics
COPY migrations /app/migrations
WORKDIR /app
USER slimlytics
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/slimlytics"]
