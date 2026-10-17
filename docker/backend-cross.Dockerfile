# syntax=docker/dockerfile:1.7
# Cross-compiles the x86-64 Linux backend on an ARM64 build host (the CI Mac mini) without
# emulation: Debian's x86-64 GCC cross toolchain links against Debian's own amd64 libssl, the
# same libssl3 the production runtime image installs.
# Usage: docker build -f docker/backend-cross.Dockerfile --output type=local,dest=OUT .
FROM rust:1.98-bookworm AS toolchain
RUN dpkg --add-architecture amd64 \
    && apt-get update \
    && apt-get install -y --no-install-recommends gcc-x86-64-linux-gnu libc6-dev-amd64-cross libssl-dev:amd64 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-gnu
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig

FROM toolchain AS builder
WORKDIR /src
COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
COPY backend/build.rs ./backend/build.rs
COPY backend/src ./backend/src
COPY docs/openapi.json ./docs/openapi.json
COPY migrations ./migrations
WORKDIR /src/backend
RUN --mount=type=cache,id=slimlytics-cross-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=slimlytics-cross-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=slimlytics-cross-backend-target,target=/src/backend/target,sharing=locked \
    touch src/lib.rs src/main.rs \
    && cargo build --locked --release --target x86_64-unknown-linux-gnu \
    && x86_64-linux-gnu-strip --strip-debug -o /slimlytics-backend \
         target/x86_64-unknown-linux-gnu/release/slimlytics-backend

FROM scratch AS export
COPY --from=builder /slimlytics-backend /slimlytics-backend
