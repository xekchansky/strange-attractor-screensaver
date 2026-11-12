#!/usr/bin/env bash
# Rebuilds the wasm bundle used by web/index.html.
#
# Requires the wasm32-unknown-unknown target and a wasm-bindgen CLI whose
# version matches the wasm-bindgen crate (0.2.x).
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/wgpu_spike.wasm
