#!/usr/bin/env bash
# Builds the WebAssembly bundle the page imports from ./pkg.
#
# Requires the wasm32-unknown-unknown target and a wasm-bindgen CLI whose
# version matches the wasm-bindgen crate. The repository's local toolchain
# (rust/.toolchain, git-ignored) is picked up automatically when present, which
# is what the agent sandbox uses; CI and a developer shell use the default one.
set -euo pipefail

web_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$web_dir/.." && pwd)"
cd "$repo_root"

cargo_bin="cargo"
wasm_bindgen_bin="${WASM_BINDGEN:-wasm-bindgen}"
if [[ -d "$repo_root/rust/.toolchain/cargo" ]]; then
  export CARGO_HOME="${CARGO_HOME:-$repo_root/rust/.toolchain/cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-$repo_root/rust/.toolchain/rustup}"
  cargo_bin="$repo_root/rust/.toolchain/cargo/bin/cargo"
  if [[ -x "$repo_root/rust/.toolchain/cargo/bin/wasm-bindgen" && -z "${WASM_BINDGEN:-}" ]]; then
    wasm_bindgen_bin="$repo_root/rust/.toolchain/cargo/bin/wasm-bindgen"
  fi
fi

"$cargo_bin" build --release -p attractor-wasm --target wasm32-unknown-unknown \
  --manifest-path "$repo_root/rust/Cargo.toml"
"$wasm_bindgen_bin" --target web --out-dir "$web_dir/pkg" \
  --out-name attractor_wasm \
  "$repo_root/rust/target/wasm32-unknown-unknown/release/attractor_wasm.wasm"

# Optional size pass; a browser bundle still works without it. The installed
# binaryen may not know every feature the bundle declares, so a failed pass
# must not fail the build the comment promises.
if command -v wasm-opt >/dev/null 2>&1; then
  if ! wasm-opt -Oz -o "$web_dir/pkg/attractor_wasm_bg.wasm" \
    "$web_dir/pkg/attractor_wasm_bg.wasm"; then
    echo "warning: wasm-opt failed; keeping the unoptimized bundle" >&2
  fi
fi
