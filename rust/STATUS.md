# Rust port — status

Design and iteration plan:
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

Protocol for a fresh session: read the design, re-run the previous
iteration's verification command, implement exactly one iteration, update this
file, and commit atomically.

## Current state

- Branch: `rust-port`
- Last completed iteration: **I0.5 — renderer and web spike** (2026-11-12,
  done with manual visual checks pending)
- Next iteration: **I1 — data model and catalog**

## Verification

From `rust/`:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same three commands in the `rust` job of
`.github/workflows/ci.yml`. The spike is deliberately outside the workspace
and outside CI; its own checks are:

```bash
cd spike
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --target wasm32-unknown-unknown --lib && ./build-wasm.sh
cargo check --target aarch64-linux-android --lib
```

The wasm target and `wasm-bindgen-cli` (0.2.130, matching the crate) are
prerequisites; `spike/README.md` has the full recipe.

## Iterations

- [x] I0 — workspace skeleton and CI
- [x] I0.5 — renderer and web spike, timeboxed (manual visual checks pending)
- [ ] I1 — data model and catalog
- [ ] I2 — formula parser
- [ ] I3 — numerical core
- [ ] I4 — engine, selection and health monitor
- [ ] I5 — view layer, no GPU
- [ ] I6 — wgpu renderer and CLI capture
- [ ] I7a — player and C ABI
- [ ] I7b — macOS host migration
- [ ] I8 — Windows `.scr`
- [ ] I9 — web demo on GitHub Pages
- [ ] I10 — Linux
- [ ] I11 — Android live wallpaper
- [ ] I12 — distribution, docs and cleanup

## Swift oracle

The oracle commit is not pinned yet. It is recorded here when I2 starts, so
the golden fixtures stay reproducible after `Core/` is retired.

## Notes and risks

- I0 created the Cargo workspace with eight empty crates and no external
  dependencies. Every crate has one smoke test asserting its package name,
  which proves the member list and manifests are wired correctly.
- The `rust` CI job runs on `ubuntu-latest`. It is independent of the Swift
  jobs, which still run on `macos-26` unchanged.
- The workflow's `push` trigger now includes `rust-port`; without it the Rust
  job would only run on pull requests and the iteration's "CI green" could
  not be observed on the working branch.
- **I0.5**: `rust/spike/` is a throwaway crate, excluded from the workspace
  and deleted in I6. It draws 10 000 ribbon segments (20 002 vertices) from
  a Lorenz orbit through one WGSL pipeline and carries the ADR for the web
  and Android backends (design doc, I0.5 section). Decisions: WebGPU with an
  automatic WebGL2 fallback; wgpu's GLES backend on Android instead of raw
  EGL; pin wgpu 30 (large API break from 24–26) and match
  `wasm-bindgen-cli` to the crate version.
- **I0.5 manual: pending.** The sandbox an agent runs in has no Metal device
  (`MTLCreateSystemDefaultDevice` returns `nil`) and cannot start a browser,
  so the two visual checks were not observed by the agent. Everything that
  can be checked without a GPU is green: 8 unit tests (including WGSL
  validation with `naga` and a full pipeline build through wgpu's `noop`
  backend), the wasm build with bindings, and `cargo check` for
  `aarch64-linux-android`. The pending checks are:

  ```bash
  cd rust/spike
  cargo run --release -- spike.png        # writes a non-blank 1280x720 PNG
  ./build-wasm.sh && python3 -m http.server 8080 --directory web
  # then open http://localhost:8080/ in Chrome and Safari
  ```

  The result is recorded here when someone runs it; it does not block I1.
- The toolchain is plain stable Rust (edition 2024); no version is pinned
  yet. I0 and I0.5 were verified locally with rustc 1.99.1 (stable). For the
  spike, a local rustup home lives in `rust/.toolchain` (git-ignored) because
  the Homebrew rustc ships only the host standard library. Pin the toolchain
  together with the wgpu version when I6 fixes the dependency set.
- wgpu 30 restructured its public API. Local examples in this repository
  (`spike/src/gpu.rs`, `spike/src/web.rs`) are the reference until I6 replaces
  the spike, because the 24–26 documentation snippets do not compile.
