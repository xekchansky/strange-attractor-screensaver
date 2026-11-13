#!/bin/bash
# Build the Linux tarball uploaded to a GitHub release.
#
# Usage: ./scripts/package_linux.sh [output-directory]
#
# Produces, in the output directory (default: ./dist):
#   strange-attractor-linux-x86_64.tar.gz
#
# The archive holds the screen saver binary, the capture/demo CLI, the
# .desktop entry and the Linux README. The binary is linked against the
# runner's glibc, so the tarball targets current distributions rather than
# anything older than the build image. The checksum is written by the release
# workflow, which covers every platform's archive with one SHA256SUMS file.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/dist"}
STAGE_NAME=strange-attractor-linux-x86_64
STAGE="$OUT/$STAGE_NAME"

mkdir -p "$OUT"
rm -rf "$STAGE"
mkdir -p "$STAGE"

echo "==> Building the release binaries"
cd "$ROOT/rust"
cargo build --release --locked -p attractor-linux-saver -p attractor-cli

echo "==> Staging"
install -m 755 target/release/attractor-linux-saver "$STAGE/"
install -m 755 target/release/attractor-cli "$STAGE/"
install -m 644 shells/linux/strange-attractor.desktop "$STAGE/"
install -m 644 shells/linux/README.md "$STAGE/README.md"

echo "==> Packing"
tar -C "$OUT" -czf "$OUT/$STAGE_NAME.tar.gz" "$STAGE_NAME"

echo "==> Done"
ls -lh "$OUT/$STAGE_NAME.tar.gz"
