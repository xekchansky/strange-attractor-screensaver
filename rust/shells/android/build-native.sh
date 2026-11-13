#!/usr/bin/env bash
# Build the Rust player as the shared library the wallpaper loads.
#
# Requires `cargo-ndk` and the Android NDK, both installed in CI (the
# `ANDROID_NDK_HOME` or `ANDROID_HOME` environment variable points at the SDK).
# The libraries land in `app/src/main/jniLibs/`, which Gradle packages into the
# APK and Git ignores.
#
# Usage: ./build-native.sh [abi ...]   (default: arm64-v8a x86_64)
set -euo pipefail

cd "$(dirname "$0")"

if [ "$#" -eq 0 ]; then
  set -- arm64-v8a x86_64
fi

targets=()
for abi in "$@"; do
  targets+=(--target "$abi")
done

cargo ndk "${targets[@]}" \
  --output-dir app/src/main/jniLibs \
  build -p attractor-ffi --release
