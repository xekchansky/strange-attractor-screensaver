# wgpu spike (I0.5, throwaway)

This crate exists to retire the two unknowns before the long port:

1. does one wgpu/WGSL pipeline draw the saver's 10 000-segment ribbon on
   Metal, in the browser, and compile for Android GLES?
2. which web and Android backends should the design commit to?

It is **not** part of the workspace (it is listed in `exclude`) and it is
deleted once `attractor-gpu` grows the real pipeline in I6. The findings are
recorded as an ADR in
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

## What it draws

A Lorenz orbit integrated with RK4 is turned into a two-vertex-per-point
triangle strip — 10 001 points, 20 002 vertices — with a per-vertex hue ramp.
The camera orbits at elevation `pi/5` and distance `radius * 2.1`, matching
the constants of the Swift `CameraController`.

## Native (macOS, Metal)

```bash
cargo run --release -- spike.png
```

The binary renders one 1280×720 frame offscreen, reads it back, writes the
PNG and prints the share of lit pixels. It exits non-zero when the frame is
blank. It needs a real GPU session; there is no software Metal fallback.

## Web (WebAssembly)

```bash
rustup target add wasm32-unknown-unknown   # once
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # once, matching wasm-bindgen
./build-wasm.sh
python3 -m http.server 8080 --directory web
```

Then open <http://localhost:8080/>. The page asks wgpu for a WebGPU adapter
and silently falls back to WebGL2 through `new_instance_with_webgpu_detection`.
Open the console to see which adapter was chosen.

## Android

```bash
rustup target add aarch64-linux-android
cargo check --target aarch64-linux-android --lib
```

`cargo check` needs no NDK: it proves the GLES/Vulkan dependency stack of
`wgpu` compiles for the target. Building an APK is I11.

## Notes

- wgpu 30 is a restructured API (`SurfaceTarget::Canvas`, `queue.present`,
  `CurrentSurfaceTexture`, `Option<&BindGroupLayout>` in pipeline layouts).
  The spike is written against 30.0.1; the ADR notes that pinning this major
  version matters.
- The toolchain used while writing the spike lived in `rust/.toolchain`
  (git-ignored) because the repository does not pin a Rust version yet.
