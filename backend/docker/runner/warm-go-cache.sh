#!/bin/sh
# Populate a read-only, world-readable Go build cache from go-warmup.go.
#
# The runner gives each job a private cache built from hard/symlinks to this
# one (see engine::link_farm), so the standard library compiles once, here, at
# image-build time — never per job, and never in a cache a job could write to.
set -eu

cache="${1:?usage: warm-go-cache.sh <cache-dir>}"
here="$(dirname "$0")"

export GOCACHE="$cache"
export GOTOOLCHAIN=local GO111MODULE=off GOPROXY=off GOENV=off GOWORK=off CGO_ENABLED=0
unset GOFLAGS

work="$(mktemp -d)"
cp "$here/go-warmup.go" "$work/main.go"
( cd "$work" && go build -o "$work/prog" main.go )
rm -rf "$work"

# The job uid must read these but never write them: a writable shared cache
# could be poisoned for the next user (Go trusts an entry by name and size).
chmod -R a-w "$cache"
find "$cache" -type d -exec chmod 0555 {} +
find "$cache" -type f -exec chmod 0444 {} +

echo "warmed Go cache at $cache: $(find "$cache" -type f | wc -l) files, $(du -sh "$cache" | cut -f1)"
