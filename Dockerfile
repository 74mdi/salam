# syntax=docker/dockerfile:1

# ---- build ---------------------------------------------------------------
FROM rust:1-slim AS build
WORKDIR /src

# Cargo layer caching via BuildKit cache mounts.
COPY Cargo.toml Cargo.lock ./
COPY koko-protocol/Cargo.toml koko-protocol/
COPY koko-core/Cargo.toml koko-core/
COPY koko-db/Cargo.toml koko-db/
COPY koko-server/Cargo.toml koko-server/
COPY koko-ui/Cargo.toml koko-ui/
COPY koko-client/Cargo.toml koko-client/
COPY koko-web/Cargo.toml koko-web/
# Dummy sources so dependency crates compile once and cache.
RUN for d in koko-protocol koko-core koko-db koko-ui koko-web; do \
        echo "" > $d/src/lib.rs 2>/dev/null || mkdir -p $d/src && echo "" > $d/src/lib.rs; \
    done && \
    echo "fn main() {}" > koko-server/src/main.rs 2>/dev/null || \
    (mkdir -p koko-server/src && echo "fn main() {}" > koko-server/src/main.rs) && \
    echo "fn main() {}" > koko-client/src/main.rs 2>/dev/null || \
    (mkdir -p koko-client/src && echo "fn main() {}" > koko-client/src/main.rs)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p koko-server && \
    cp target/release/koko-server /koko-server

# Real sources: rebuild only the workspace crates.
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    touch koko-protocol/src/lib.rs koko-core/src/lib.rs koko-db/src/lib.rs koko-server/src/main.rs && \
    cargo build --release -p koko-server && \
    cp target/release/koko-server /koko-server-final

# ---- runtime --------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /koko-server-final /usr/local/bin/koko-server

# One SQLite file + one media dir, both under /data (mount a volume!).
ENV KOKO_DB=/data/koko.db \
    KOKO_MEDIA_DIR=/data/media \
    KOKO_BIND=0.0.0.0:8080
VOLUME /data
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["koko-server"]
