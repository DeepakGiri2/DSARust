# syntax=docker/dockerfile:1
#
# dsa-runner: one image, two transports (http for compose/ECS, lambda for AWS).
#
#   docker build -f backend/docker/runner.Dockerfile -t dsa-runner .
#   docker build -f backend/docker/runner.Dockerfile --target test -t dsa-runner-test . \
#     && docker run --rm dsa-runner-test      # the sandbox tests, as root
#
# Build context is the repository root, but only `backend/` is copied: the
# runner depends on `dsa-protocol` and nothing under `../crates`, so the
# desktop app's sources never enter the image. The workspace's `api` member IS
# pulled out at build time (it does depend on `../crates`), which is a build
# concern only and never touches the tracked manifest.
#
# The same image runs as root under compose/ECS (so it can switch to per-job
# uids) and as an arbitrary non-root uid under Lambda (read-only rootfs except
# /tmp). Everything written at runtime lives under /tmp; everything read from
# the image is world-readable.

ARG RUST_VERSION=1.97
ARG DEBIAN_RELEASE=trixie
ARG GO_VERSION=1.27.1
# From https://go.dev/dl/?mode=json — bump together with GO_VERSION.
ARG GO_SHA256_AMD64=63d339f0da5ab53635a56f2490a7984dfe12dfcff22ad749f63edaf590168445
ARG GO_SHA256_ARM64=3450b45a3f9ee8568792736a5c5e70a1f2e9b36c35a8f74958c03e51d7d92bec

# ── toolchains: the runtime's language stack, shared by the final and test
#    images so it is installed and pinned exactly once ─────────────────────────
FROM debian:${DEBIAN_RELEASE}-slim AS toolchains
ARG GO_VERSION
ARG GO_SHA256_AMD64
ARG GO_SHA256_ARM64
ENV DEBIAN_FRONTEND=noninteractive
# g++ (with its headers), a headless JDK (javac + java), Python, and curl +
# CA roots so the Go tarball can be fetched and verified. The layer is not
# cleaned separately: `--mount=type=cache` keeps the apt lists out of the image.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update \
 && apt-get install -y --no-install-recommends \
      g++ \
      openjdk-21-jdk-headless \
      python3 \
      curl \
      ca-certificates

# The Go toolchain comes from the official tarball, not apt, so the version is
# pinned and its checksum verified here — on every architecture: a build for
# one without a pinned checksum fails rather than installing it unverified.
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
      amd64) goarch=amd64; sum="${GO_SHA256_AMD64}" ;; \
      arm64) goarch=arm64; sum="${GO_SHA256_ARM64}" ;; \
      *) echo "unsupported architecture: $arch" >&2; exit 1 ;; \
    esac; \
    [ -n "$sum" ] || { echo "no pinned Go checksum for $goarch" >&2; exit 1; }; \
    url="https://go.dev/dl/go${GO_VERSION}.linux-${goarch}.tar.gz"; \
    curl -fsSL "$url" -o /tmp/go.tar.gz; \
    echo "${sum}  /tmp/go.tar.gz" | sha256sum -c -; \
    tar -C /usr/local -xzf /tmp/go.tar.gz; \
    rm /tmp/go.tar.gz; \
    /usr/local/go/bin/go version
ENV PATH=/usr/local/go/bin:$PATH

# ── warm: build the shared, read-only Go build cache ─────────────────────────
FROM toolchains AS warm
COPY backend/docker/runner/go-warmup.go backend/docker/runner/warm-go-cache.sh /warm/
RUN sh /warm/warm-go-cache.sh /opt/dsa-runner/go-cache && rm -rf /warm

# ── builder: compile the runner, dependencies cached in their own layer ───────
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS builder
WORKDIR /build
# Drop the `api` member so the workspace resolves from `backend/` alone. This
# rewrites only the throwaway copy inside the image.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN sed -i 's#"crates/api", ##' Cargo.toml
COPY backend/crates/protocol/Cargo.toml crates/protocol/
COPY backend/crates/runner/Cargo.toml crates/runner/
# A stub build compiles every dependency into a layer keyed on the manifests
# and lockfile, so editing the runner's own sources does not recompile them.
RUN mkdir -p crates/protocol/src crates/runner/src \
 && : > crates/protocol/src/lib.rs \
 && : > crates/runner/src/lib.rs \
 && echo 'fn main() {}' > crates/runner/src/main.rs \
 && cargo build --release -p dsa-runner \
 && rm -rf crates/protocol/src crates/runner/src
COPY backend/crates/protocol/src crates/protocol/src
COPY backend/crates/runner/src crates/runner/src
# Bust the stub fingerprint so the real sources are picked up, then build.
RUN touch crates/protocol/src/lib.rs crates/runner/src/lib.rs crates/runner/src/main.rs \
 && cargo build --release -p dsa-runner \
 && strip target/release/dsa-runner

# ── base: the runtime filesystem, shared by the test and runtime stages ─────
FROM toolchains AS base
# Everything the runner writes goes here; on Lambda /tmp is the only writable
# mount, and this default keeps compose/ECS identical.
ENV RUNNER_WORK_DIR=/tmp/dsa-runner \
    RUNNER_GO_WARM_CACHE=/opt/dsa-runner/go-cache
# No setuid/setgid binaries (su, mount, passwd, …). Jobs run with
# no_new_privs, which already makes those bits inert; this is the second lock.
RUN find / -xdev -type f -perm /6000 -exec chmod ug-s {} + \
 && ! find / -xdev -type f -perm /6000 | grep -q .
COPY --from=warm /opt/dsa-runner/go-cache /opt/dsa-runner/go-cache
COPY --from=builder /build/target/release/dsa-runner /usr/local/bin/dsa-runner

# ── test: the runtime filesystem + Rust, to run the sandbox integration tests
#    inside a real Linux container. Built with `--target test`; never shipped ─
FROM base AS test
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/go/bin:/usr/bin:/bin \
    CARGO_TARGET_DIR=/build/target
COPY --from=builder /usr/local/cargo /usr/local/cargo
COPY --from=builder /usr/local/rustup /usr/local/rustup
WORKDIR /build
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN sed -i 's#"crates/api", ##' Cargo.toml
COPY backend/crates/protocol crates/protocol
COPY backend/crates/runner crates/runner
# Compile the tests now (network available), so `docker run` executes them
# offline and can be repeated.
RUN cargo test -p dsa-runner --no-run
CMD ["cargo", "test", "-p", "dsa-runner", "--", "--nocapture", "--test-threads=4"]

# ── runtime: the shipped image. It must stay the LAST stage: a build that
#    names no --target gets the last one, and the runner must never ship as
#    the test image by accident ────────────────────────────────────────────────
FROM base AS runtime
# The runner takes no arguments (it reads the environment and serves), so there
# is no in-image smoke test that would not block; the `test` stage exercises it.
EXPOSE 8081
ENTRYPOINT ["/usr/local/bin/dsa-runner"]
