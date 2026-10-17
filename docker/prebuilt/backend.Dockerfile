# Production backend image from a binary cross-compiled in CI (docker/backend-cross.Dockerfile).
# Same runtime as docker/backend.Dockerfile, so the base layers are shared on the host.
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --create-home slimlytics
COPY slimlytics-backend /usr/local/bin/slimlytics
COPY migrations /app/migrations
WORKDIR /app
USER slimlytics
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/slimlytics"]
