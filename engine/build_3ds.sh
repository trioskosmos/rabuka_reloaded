#!/usr/bin/env bash
# Build rabuka_engine harness for 3DS using a docker image (host must have Docker)
set -euo pipefail
ROOT_DIR=$(cd "$(dirname "$0")"/.. && pwd)

docker build -t rabuka-engine-3ds -f "$ROOT_DIR/docker/Dockerfile" "$ROOT_DIR"

echo "Docker image built. To run the build and extract artifacts, run:"
echo "  docker run --rm -v \"$ROOT_DIR\":/work rabuka-engine-3ds bash -lc 'cd /work/engine && cargo build --bin harness --release --no-default-features'"
