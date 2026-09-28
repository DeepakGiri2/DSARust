# syntax=docker/dockerfile:1.7
#
# dsa-api — the stateless HTTP API.
#
# Build context is the repository root: the API links the desktop's trace
# engine from crates/ and bakes content/ into the image, so the image that
# serves a problem is the image that knows its animation.
#
#   docker build -f backend/docker/api.Dockerfile -t dsa-api .
#
# The same image runs `dsa-api serve` (the service) and `dsa-api migrate` (the
# one-off task before each rollout).

ARG RUST_VERSION=1.97

FROM rust:${RUST_VERSION}-bookworm AS build
WORKDIR /src
# crates/dsa-* inherit their versions and dependency specs from the desktop
# workspace manifest, so it has to be present even though the backend is its
# own workspace with its own lockfile.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY backend ./backend
COPY content ./content
WORKDIR /src/backend
# The cache mounts keep the registry and target directory between local
# builds; the binary is copied out in the same step because a cache mount is
# not part of the layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/backend/target \
    cargo build --release --locked -p dsa-api --features aws \
    && install -m 0755 target/release/dsa-api /usr/local/bin/dsa-api
# Refuse to produce an image whose content does not load — the same check
# the server runs at boot, moved to where a failure costs nothing.
RUN DSA_CONTENT_DIR=/src/content dsa-api check-content

# No shell, no package manager: nothing for an attacker to use if the process
# is ever compromised. glibc and CA roots are all the binary needs.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/dsa-api /usr/local/bin/dsa-api
COPY --from=build /src/content /app/content
ENV DSA_CONTENT_DIR=/app/content \
    DSA_BIND=0.0.0.0:8080 \
    DSA_METRICS_BIND=0.0.0.0:9090 \
    LOG_FORMAT=json
USER 10001:10001
EXPOSE 8080 9090
ENTRYPOINT ["dsa-api"]
CMD ["serve"]
