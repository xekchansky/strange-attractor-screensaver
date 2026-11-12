# Rust port — status

Design and iteration plan:
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

Protocol for a fresh session: read the design, re-run the previous
iteration's verification command, implement exactly one iteration, update this
file, and commit atomically.

## Current state

- Branch: `rust-port`
- Last completed iteration: **I1 — data model and catalog** (2026-11-12)
- Next iteration: **I2 — formula evaluation and golden fixtures**

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
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
cargo build --release --target wasm32-unknown-unknown --lib && ./build-wasm.sh
cargo check --target aarch64-linux-android --lib
```

`web.rs` is compiled only for wasm, so the host clippy pass does not see it;
the second clippy line covers it.

The wasm target and `wasm-bindgen-cli` (0.2.130, matching the crate) are
prerequisites; `spike/README.md` has the full recipe.

## Iterations

- [x] I0 — workspace skeleton and CI
- [x] I0.5 — renderer and web spike, timeboxed
- [x] I1 — data model and catalog
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
- **I1**: `attractor-core` now mirrors the Swift data model (`MinMax<Double>`
  → `MinMax<f64>`, `MinMax<Int>` → `MinMax<i64>`, so a negative `trailLength`
  decodes and `validate` drops only that descriptor instead of failing the
  whole catalog), decodes the catalog with serde (`include_str!` of the Swift
  resource, so the path in `catalog.rs` points outside the crate) and ports
  both validation suites: `tests/catalog_decoding.rs` and
  `tests/catalog_validation.rs`. Range messages and issue ordering follow the
  Swift code; formula errors use Rust's human-readable `Display` text rather
  than Swift's enum-case names.
- **I1 parser depth cap**: `formula::MAX_PARSE_DEPTH` (256) rejects
  pathological nesting that would overflow the stack. This is a Rust-only
  safety limit and a deliberate deviation from Swift, whose parser has no cap
  and aborts on the same input; no formula the Swift oracle accepts is
  rejected. The error variant `FormulaError::TooDeep` carries it.
- **I1 ruling**: catalog validation must reject formula syntax errors,
  unsupported functions and undeclared variables, which needs the AST and
  parser that the design lists under I2. I1 therefore ships
  `formula::Expression`, its function table, `referenced_variables()` /
  `referenced_functions()` and the recursive-descent parser (syntax only).
  I2 keeps evaluation, domain errors and the golden fixtures; the design
  doc's I1/I2 sections say so.
- **Sandbox note**: in the agent sandbox `~/.cargo` is not writable, so local
  builds point `CARGO_HOME` at the git-ignored `rust/.toolchain/cargo` and
  `RUSTUP_HOME` at `rust/.toolchain/rustup`. CI and a normal developer shell
  use the default locations; nothing in the repository depends on this.
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
- **I0.5 manual: verified 2026-11-12.** The native frame rendered on an
  Apple M4 Pro (`adapter: Apple M4 Pro (Metal, IntegratedGpu)`) and wrote a
  1280x720 PNG with 5.40% of pixels lit. The wasm page rendered in Chrome;
  two DevTools screenshots taken 2.5 s apart differed, so the camera
  animates. Two defects surfaced and were fixed in this pass:
  - `lit_pixels` compared the linear clear color against the sRGB-encoded
    readback, so every background pixel counted as lit (the first run
    reported 100%). It now encodes the clear color with the sRGB transfer
    function; tests pin the stored bytes (33, 33, 48).
  - the web loop subtracted `Date.now()` epoch milliseconds from the rAF
    timestamp (`performance.now()`), producing a huge offset at which `f32`
    seconds lose all resolution. The camera angle stuck and the page looked
    frozen. `FrameClock` now takes its origin from the first frame timestamp.
- **Sandbox configuration for the checks.** A macOS Seatbelt sandbox cannot
  start a browser or reach Metal. The git-ignored
  `.claude/settings.local.json` lists `cargo run *` and
  `./rust/spike/web-check.sh *` under `sandbox.excludedCommands` and allows
  the crates.io domains, so an agent can reproduce both checks. Pipes and
  `&&` chains drop the exclusion: each part of a call must match an entry.
  Headless `--virtual-time-budget` does not drive rAF (three frames, 17 ms
  observed), so animation is verified with a live DevTools session:

  ```bash
  ./rust/spike/web-check.sh --serve "http://localhost:8080/?v=1" \
    --remote-debugging-port=9222   # then capture two frames over CDP
  ```

  A single-frame screenshot still works and proves the page draws:

  ```bash
  ./rust/spike/web-check.sh http://localhost:8080/ shot.png \
    --virtual-time-budget=4000
  ```
- The toolchain is plain stable Rust (edition 2024); no version is pinned
  yet. I0 and I0.5 were verified locally with rustc 1.99.1 (stable). For the
  spike, a local rustup home lives in `rust/.toolchain` (git-ignored) because
  the Homebrew rustc ships only the host standard library. Pin the toolchain
  together with the wgpu version when I6 fixes the dependency set.
- wgpu 30 restructured its public API. Local examples in this repository
  (`spike/src/gpu.rs`, `spike/src/web.rs`) are the reference until I6 replaces
  the spike, because the 24–26 documentation snippets do not compile.
