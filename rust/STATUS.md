# Rust port — status

Design and iteration plan:
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

Protocol for a fresh session: read the design, re-run the previous
iteration's verification command, implement exactly one iteration, update this
file, and commit atomically.

## Current state

- Branch: `rust-port`
- Last completed iteration: **I11 — Android live wallpaper** (2026-11-14;
  CI green, run 43514690865, the emulator/phone pass pending)
- Last completed change: **I11 — Android** (2026-11-14)
- Next iteration: **I12 — distribution, docs and cleanup**
- Windows manual checklist: `crates/attractor-windows-saver/README.md`,
  scheduled with the reviewer after I8 (design §6).
- Linux manual checklist: `shells/linux/README.md`, scheduled with the
  reviewer after I10 (design §6).
- Android manual checklist: `shells/android/README.md`, scheduled with the
  reviewer after I11 (design §6).
- Pre-migration visual reference: **`ef3d61b`**, the last commit whose
  `.saver` still rendered through SceneKit. The Rust host must match it
  side by side (manual checklist below).

## Verification

From `rust/`:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same three commands in the `rust` job of
`.github/workflows/ci.yml`.

The headless capture needs a real GPU adapter and therefore a GUI session:

```bash
cargo run -p attractor-cli -- --attractor lorenz --seed 42 \
  --screenshot out.png --frames 120
```

The macOS host links the Rust player as a static library, so its build first
compiles `attractor-ffi` for every architecture in `ARCHS`:

```bash
cd Host && xcodegen generate
cd .. && xcodebuild -project Host/AttractorHost.xcodeproj -scheme AttractorSaver \
  -configuration Release -derivedDataPath "$TMPDIR/host-dd" ARCHS=arm64 build
```

`ARCHS=arm64` keeps the local check single-architecture. A universal Release
build needs the `x86_64-apple-darwin` standard library in the Rust toolchain;
the build phase fails with an actionable message when it is missing.

The Windows shell is cross-checked from the macOS host against the GNU
target's standard library (`rustup target add x86_64-pc-windows-gnu`); both
commands compile and lint the Win32 code without linking it:

```bash
cd rust
cargo check -p attractor-windows-saver --target x86_64-pc-windows-gnu --all-targets
cargo clippy -p attractor-windows-saver -p attractor-gpu \
  --target x86_64-pc-windows-gnu --all-targets -- -D warnings
```

The `.scr` self-test renders through the shared player and therefore runs on
every platform, which is how the Windows path is smoke-checked locally:

```bash
cargo run -p attractor-windows-saver -- --self-test
cargo run -p attractor-windows-saver -- --self-test --noop
```

The Linux shell is cross-checked from the macOS host against the Linux
target's standard library (the X11 and winit code only compiles there; the
noop self-test runs on every OS):

```bash
cd rust
cargo clippy -p attractor-linux-saver -p attractor-gpu \
  --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
cargo run -p attractor-linux-saver -- --self-test --noop
```

The CI `linux` job installs Xvfb and Mesa's Vulkan driver, requires the
adapter self-test to draw ink (a skip is a failure there), runs the
xscreensaver `-root` mode and the fullscreen runner for 30 frames each under
Xvfb, and captures a PNG through `attractor-cli`. Native Wayland has no CI
runner and stays on the manual checklist.

The Android shell is cross-checked from the macOS host against the Android
target's standard library, and its APK builds with the SDK and JDK 17:

```bash
cd rust
cargo clippy -p attractor-ffi -p attractor-gpu \
  --target aarch64-linux-android --all-targets -- -D warnings

cd shells/android
rustup target add aarch64-linux-android x86_64-linux-android
cargo install --locked cargo-ndk        # once; needs the NDK
./build-native.sh                       # jniLibs for the APK
./gradlew assembleDebug lint
```

The JNI names are compared with the Kotlin declarations on the host by
`cargo test -p attractor-ffi`, so a typo fails the ordinary workspace run
instead of waiting for a device. The native library cannot be built without
the NDK; CI installs one and builds the libraries before Gradle, so the APK
artifact there is complete.

The web demo builds its wasm bundle and runs the browser suite:

```bash
bash web/build.sh
cd web
npm ci
npx playwright install chromium
npm test
```

The agent sandbox cannot start Chromium (Seatbelt denies the Mach-port
rendezvous, `bootstrap_check_in … Permission denied (1100)`), so a local
Playwright run there fails before the page loads; CI's `web` job is the
browser gate. The static server alone does work in the sandbox.

The Swift oracle checks (in the agent sandbox SwiftPM needs
`--disable-sandbox`; a normal shell does not):

```bash
cd Core && swift test --disable-sandbox
cd Demo && swift build --disable-sandbox
```

The pipeline tests build through wgpu's `noop` backend, so they run without a
GPU; the adapter-backed render tests (`headless_frame_is_not_blank`, the
player render/parity tests, the FFI native render and `tests/text.rs`) skip
with a printed reason when no adapter is available. Locally a bare
`cargo test --workspace` runs them on Metal, because the project sandbox
excludes `cargo test` (see the sandbox note at the end). The spike was
deleted in I6, and with it the wasm32/android check recipes it carried; I9
and I11 add them back against `attractor-wasm` and the Android shell.

## Iterations

- [x] I0 — workspace skeleton and CI
- [x] I0.5 — renderer and web spike, timeboxed
- [x] I1 — data model and catalog
- [x] I2 — formula evaluation and golden fixtures
- [x] I3 — numerical core
- [x] I4 — engine, selection and health monitor
- [x] I5 — view layer, no GPU
- [x] I6 — wgpu renderer and CLI capture
- [x] I7a — player and C ABI
- [x] I7b — macOS host migration (done; manual: preview + fullscreen
  verified 2026-11-12, multi-display pending)
- [x] I8 — Windows `.scr` (done; manual: pending, checklist in the crate
  README)
- [x] I9 — web demo on GitHub Pages (done; manual: Pages enablement and the
  Chrome/Safari/Firefox pass pending)
- [x] I10 — Linux (done; CI green, run 43507189418; manual: Debian/Ubuntu
  X11 and Wayland checklist in `shells/linux/README.md`)
- [x] I11 — Android live wallpaper (done; CI green, run 43514690865; manual:
  emulator/phone checklist in `shells/android/README.md`)
- [ ] I12 — distribution, docs and cleanup

## Swift oracle

Pinned at **`dfbadb8`**, the last commit that touched `Core/Sources`. The
golden fixtures were generated from that tree on 2026-11-12. The generators
(`Core/Tests/AttractorCoreTests/FormulaFixtureDumpTests.swift`, added in the
I2 commit, and `NumericalFixtureDumpTests.swift`, added in the I3 commit) sit
on top of `dfbadb8`; to reproduce the fixtures, check out `dfbadb8`, copy the
generators in, and run the commands in
`rust/crates/attractor-core/tests/fixtures/README.md`. After I12 retires
`Core/`, the oracle tag will contain the generators, so the checkout alone is
enough.

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
- **I2**: `Expression::evaluate(&HashMap<String, f64>)` ports the Swift
  semantics exactly: the divisor is evaluated before the dividend (so a zero
  divisor wins over an unknown variable on the left), `sqrt` rejects negative
  arguments and NaN, `ln`/`log` are the natural logarithm and reject
  non-positive arguments and NaN, and a negative base with a fractional
  exponent returns NaN rather than an error. `tests/formula_evaluation.rs`
  ports `ExpressionTests` plus the error-precedence and domain rules.
- **I2 fixtures**: `tests/fixtures/formula_evaluation.json` holds 288 entries
  (12 attractors × 3 formulas × 8 sample states) generated by the Swift
  oracle; `tests/formula_fixtures.rs` re-evaluates them with a `1e-12`
  relative tolerance (absolute floor `1e-12`) and also checks that the
  fixture expressions and bound variables still match the catalog. Values are
  written with `%.17g`, so they round-trip exactly. `tests/fixtures/README.md`
  documents the generator command. The Swift generator suite is skipped
  unless `ATTRACTOR_DUMP_FIXTURES=1`, so a normal `swift test` writes nothing.
- **I2 ruling**: the oracle is pinned to `dfbadb8` (last `Core/Sources`
  commit) rather than to the rust-port HEAD, because the fixtures depend on
  Swift sources only; the generator itself lands in the I2 commit. Cost if
  wrong: regenerating needs one extra file copy until I12 archives the
  generator with the tag.
- **I2 deferred (minor, from review)**: catalog formulas only call `sin`,
  `abs`, `^` and arithmetic, so `exp`/`cos`/`tan`/`sqrt`/`ln`/`log` have only
  trivial self-referential unit coverage. Add synthetic formulas to the
  generator if cross-platform libm fidelity of those functions becomes a
  requirement. The domain edges that used to be untested — `ln(NaN)`,
  `sqrt(-0.0)`, a `-0.0` divisor and the dividend side of division precedence
  (`1 / q`) — now have unit tests in `tests/formula_evaluation.rs`.
- **I3**: the numerical core is ported in `ode_solver`, `splitmix64`,
  `random`, `randomizer`, `simulation_step_scheduler`, `trace_buffer` and
  `restart_limiter`; `AttractorSelection` (the plain data aggregate) also
  lands here because it is the randomizer's return type. `AttractorModel`,
  `DebugConfig` and the engine stay in I4, which adds model building to the
  selection.
- **I3 deterministic RNG**: `SplitMix64` is bit-identical to Swift.
  `RandomSource` carries the Swift standard library's range-drawing algorithms
  as provided methods — Lemire's `next(upperBound:)`, `Int.random(in:)` and
  the 53-bit `Double.random(in:using:)` — so the randomizer consumes the same
  draws in the same order as Swift (parameters in declaration order, then the
  five hyperparameters, then x/y/z). `tests/randomizer_fixtures.rs` proves it
  bit for bit for 16 seeds over the whole catalog, not just that values fall
  in range. The entropy-backed Swift overloads (`select` without a generator)
  are not ported: every entry point takes a `RandomSource`, and the player
  picks a seeding source in I7a.
- **I3 numerical fixtures**: `tests/fixtures/trajectories.json` holds 1000
  RK4 steps for Lorenz, Chua and Aizawa plus 500 Euler steps for Lorenz, all
  from `DebugConfig` midpoints and the offset start; the Rust integration
  matches within `1e-9` relative (absolute floor `1e-9`).
  `tests/trajectory_fixtures.rs` also runs 20 000-step long-run invariants
  (finite, bounded, still moving) and checks both methods stay finite on
  Lorenz. The tolerance is safe because the compared horizon is short; long
  horizons are covered only by invariants, as the numeric contract requires.
- **I3 buffer tests**: `TraceBuffer` exposes `len`, `Index`, `iter`, `first`
  and `last` instead of Swift's collection protocol; its property test
  compares 200 random appends per capacity (1–8) against a reference
  `VecDeque`. No `proptest` dependency was added.
- **I3 ruling**: the design listed `AttractorSelection` under I4, but the
  randomizer returns one, so the aggregate moved to I3. Cost of the split:
  `build_model` and the engine wait for `AttractorModel`, which is still I4.
- **I3 oracle caveat**: Swift documents that its standard-library random
  algorithms may change between compiler versions. The fixtures pin Swift
  6.4.2 behaviour; re-run the generator if the oracle toolchain changes.
- **I4**: the engine, model, debug configuration and health monitor are ported
  in `attractor_model`, `attractor_engine`, `debug_config` and
  `simulation_health_monitor`; `AttractorSelection::build_model` completes the
  aggregate from I3. `tests/attractor_engine.rs`, `tests/attractor_model.rs`,
  `tests/debug_config.rs` and `tests/simulation_health_monitor.rs` port the
  four Swift suites one-to-one (16 health-monitor tests, including the Aizawa
  laminar phases, the reported Chua ring and the non-integer-period cycle).
  Shared test descriptors live in `tests/support/mod.rs`.
- **I4 rulings (API shape)**:
  - Swift has two error types (`AttractorModelError` and `FormulaError`);
    Rust merges the parse failure into
    `AttractorModelError::Formula(FormulaError)` and implements
    `std::error::Error::source` for it, so `?` works in one place and callers
    still get both variants.
  - Swift default arguments (`parameters: [:]`, `settings: nil`) become
    `DebugConfig::selection` (midpoints everywhere) and
    `DebugConfig::selection_with` (explicit parameters and/or settings).
  - `SimulationHealthThresholds` keeps its public fields but sanitizes a copy
    in `SimulationHealthMonitor::new`; `check_count()` replaces Swift's
    `private(set)` property that `@testable` tests read. Step counts are
    `usize`, so a negative threshold is unrepresentable — the Swift
    sanitization test's negative inputs become zeros/`NaN`/an oversized window.
  - `AttractorEngine` exposes `state()` and `trace()` accessors instead of
    Swift's `private(set)` stored properties.
- **I4 fixture**: `tests/fixtures/engine_traces.json` records, for seeds
  0…15, the Swift randomizer selection and a 200-step RK4 `AttractorEngine`
  trace with a trail capacity of 128 (so the ring wraps). The Rust
  `tests/engine_fixtures.rs` re-selects from the seed, compares the selection
  bit-for-bit, then replays the trajectory within `1e-9` relative and checks
  the final trace against the newest 128 fixture states. This is the
  end-to-end determinism gate from the design's numerical contract.
  `NumericalFixtureDumpTests.dumpEngineTraces` writes the file; it was
  generated with the same standalone `swiftc` recipe as the I3 fixtures
  because SwiftPM's nested sandbox is unavailable here. The committed
  generator parses (`swiftc -parse`) and has the same body as the program that
  produced the JSON.
- **I4 performance note**: `AttractorModel::rate` clones the parameter map per
  evaluation, mirroring Swift, which builds the variable dictionary the same
  way. RK4 evaluates the field four times per step; the catalog test
  (12 attractors × 4 starts × 8 000 steps) still finishes in about four
  seconds in a debug build. Optimize only if profiling in I6 shows the formula
  layer as a bottleneck.
- **I5**: the GPU-free view layer lives in `attractor-view`: `Vector3` and
  `Mat4` (column-major look-at), `CameraController` (the Swift orbit
  constants, six-frame framing cache, 6% easing, `f32`-rounded position),
  `TrailBuilder`/`TrailGeometry` (16 384-point budget, uniform stride, head
  preservation, world-space ribbon radius, all four color modes) and
  `OverlayModel`. Tests: `tests/math.rs`, `tests/camera.rs`,
  `tests/trail.rs`, `tests/overlay.rs` and the golden
  `tests/view_fixtures.rs`.
- **I5 fidelity**: Swift's `SCNNode.position` getter and `SCNGeometrySource`
  store `Float32` even though the API is `CGFloat`-typed (`SCNVector3` itself
  keeps `f64`; verified with a probe). The Rust camera rounds its position
  through `f32` on every update, and the ribbon rounds vertex positions and
  colors through `f32` at the same points, so the committed fixture matches
  bit for bit. The camera's center, distance and azimuth stay `f64`, like the
  Swift controller's stored properties.
- **I5 fixture**: `tests/fixtures/view_fixtures.json` records fourteen camera
  positions around a fixed nine-point trace and the complete
  position/color/index buffers for five trail cases (`trail` at offsets 0.0
  and 0.3, `age`, `depth`, `spatial` at 0.3). The generator is
  `Render/Tests/AttractorRenderTests/ViewFixtureDumpTests.swift`, enabled by
  the new `AttractorRenderTests` target in `Render/Package.swift` and
  disabled unless `ATTRACTOR_DUMP_FIXTURES=1`; `tests/fixtures/README.md` has
  the command. SwiftPM cannot start its nested sandbox in the agent
  environment, so the committed file was produced by compiling the same
  source bodies with `swiftc`; the test target type-checks and the package
  builds with `--disable-sandbox`, which CI does not need.
- **I5 rulings**:
  - The design calls 16 384 a "vertex budget"; Swift counts sampled trace
    points, so a fully populated ribbon reaches 32 770 vertices. The Rust
    builder keeps Swift's meaning (`TrailBuilder::DEFAULT_BUDGET` counts
    points).
  - `CameraController::random` takes an explicit `RandomSource`, because the
    Swift initializer reaches into the system RNG, which is not seedable. The
    draw mirrors `Double.random(in: 0..<2π)` (53 bits below 2^53). The
    screensaver's random angle is not part of the seeded determinism
    contract, so this is not a parity risk; captures use
    `CameraController::new` with a pinned angle.
  - The ribbon builder is stateful (`build` refills its buffers in place)
    rather than a free function, matching `TraceRenderer`'s reuse rationale;
    `TrailGeometry` owns the three buffers and exposes read-only accessors.
- **I5 deferred (minor)**: `Mat4::look_at` has no perspective projection; I6
  adds it with the aspect ratio (done). `OverlayModel` spells non-finite
  values the way C does (`nan`, `inf`) for parity; a test now covers a
  non-finite state in `tests/overlay.rs`.
- **I6**: the GPU path and the CLI exist. `attractor-gpu` owns a headless
  `GpuContext` (native backends; `new_noop` for CI), the ribbon pipeline,
  a 4x multisampled sRGB target resolved into a single-sampled texture, frame
  readback, PNG writing and `lit_pixels`; `attractor-cli` owns flag parsing,
  `SimulationSession` (engine + camera + health monitor + ribbon) and the
  capture loop. Dependencies are wgpu 30, glyphon 0.12 (cosmic-text 0.19),
  bytemuck, pollster, png, env_logger. The spike was deleted as the design
  required.
- **I6 projection fidelity**: `Mat4::perspective` uses clip-space depth in
  `[0, 1]` (wgpu convention, right-handed camera looking down `-z`).
  `CameraController::view_projection(aspect)` applies SceneKit's default
  60-degree field of view along the larger frame dimension, so landscape
  frames get it horizontally and portrait frames vertically. Near/far planes
  are scaled to the current orbit distance (0.01x and 100x) because the Swift
  camera auto-adjusts its z-range; clipping planes do not change projected x/y
  for geometry inside the frustum, so this is visually equivalent.
- **I6 ribbon and text**: the vertex format is `vec3 position + vec3 linear
  color`; the strip is drawn directly from the vertex buffer because
  `TrailGeometry::indices` is `0, 1, 2, …` in strip order. The target is
  `Rgba8UnormSrgb` with a black clear, so linear colors are encoded on write
  like SceneKit's sRGB surface. The overlay is glyphon with a monospace face
  at 13 pt / 16 px line height, 16 pt margins, white, bottom-left; font size
  and margin multiply by `--capture-scale`, matching a Retina snapshot of the
  13 pt Swift label. Text lives behind the default-on `text` feature so the
  wasm build (DOM overlay) can disable it in I9; a missing font degrades to a
  logged warning, not a failed capture.
- **I6 CLI**: flag parity with the Swift demo plus `--seed`, `--frames`,
  `--width` and `--height`. Defaults: 60 000 warm-up steps, 90 settle frames
  before a capture, 1200x900 logical points, `--capture-scale 2`,
  `--record` at 4 s and stride 2. The settle phase is shared by both capture
  modes, exactly as in the Swift `CaptureController`: without it the first
  recorded seconds show the camera zooming into place. `--frames` is the
  screenshot settle count or,
  with `--record`, the number of frames to record; it conflicts with
  `--record-seconds`. Exit codes follow the Swift demo: 2 for usage errors
  (unknown flag/attractor/parameter, bad value, bad settings, missing capture,
  oversized capture), 1 for catalog/model/integration/health/GPU/IO failures.
  A run without `--screenshot`/`--record` exits 2 with usage until the
  interactive player lands in I7.
- **I6 deterministic snapshot**: `crates/attractor-cli/tests/fixtures/cli_lorenz_snapshot.json`
  pins a seeded end-to-end run (seed 42, 5 000 warm-up steps, 60 frames) as
  exact `f32` bits: vertex counts, ribbon radius, selected vertices, an FNV-1a
  digest of every position and color, plus the sampled parameters and initial
  state. Camera azimuth and elevation are pinned to zero and `--cam-speed` /
  `--trail-color-speed` to zero, so the projection uses exact `sin(0)`/`cos(0)`
  values and the snapshot stays bit-identical across libm implementations.
  Floats are stored as bit patterns because serde_json's default float parser
  is fast but not always correctly rounded; decimal round-trips are not a
  valid exact contract. Regenerate with
  `ATTRACTOR_DUMP_FIXTURES=1 cargo test -p attractor-cli --test headless_snapshot`.
- **I6 rulings**:
  - The real-adapter render tests skip (with a printed reason) when no GPU
    adapter exists. CI's `rust` job runs on `ubuntu-latest` without a GPU, so
    the deterministic gate there is the geometry snapshot plus the noop
    pipeline build; the visual gate is the manual `cargo run` from the design's
    verification command (verified on an Apple M4 Pro: 2400x1800, 872 368 lit
    pixels, 120 frames).
  - The design's secondary "loose perceptual diff against a committed
    reference image" is deferred until CI has a GPU runner; committing a
    multi-megabyte GPU-specific PNG now would add weight without a test that
    can run. The design's I6 section notes this.
  - The headless loop rebuilds the ribbon every frame, whereas the Swift scene
    rebuilds every second tick. The throttle exists to relieve live SceneKit
    redraw pressure; every captured frame in the Swift path (even tick or
    stride-aligned) already has a fresh ribbon, so captures match and the
    headless path stays simpler.
  - A failed integration step during warm-up is fatal (exit 1). The Swift
    capture logs it and continues rendering the remaining trail; the Rust
    policy treats any non-evaluable run as degenerate, which is what
    `docs/capture_all.sh` wants (retry instead of publishing a partial image).
- **I6 deferred (minor, all closed)**: the overlay now has a pixel-level test
  (`the_overlay_draws_glyphs_into_the_lower_left_corner`: a blank frame has no
  ink, an overlay frame inks the lower half); `--frames` is capped at
  `CommandLineArguments::MAXIMUM_FRAMES` (216 000, one hour at 60 fps), so a
  mistyped value is a usage error like an oversized `--record-seconds`; the
  fixture README now states the capture flags a manual CLI reproduction needs.
  The two renderer gaps — no `resize` and the broken single-sample resolve
  target — were fixed in I7a.
- **I7a**: `attractor-player` now owns the runtime and `attractor-cli` is one
  more shell over it. `Player::{new, warm_up, tick, restart, resize, render}`
  supersedes the CLI's `SimulationSession`, which is deleted. The player keeps
  the validated descriptor list for restarts and owns a `SplitMix64`, so the
  same seed draws the same camera and replacement selections. `tick` returns
  `TickEvent::{Advanced, Restarted, RestartDeferred}`; with
  `PlayerOptions::heal_automatically` a health-monitor verdict replaces the
  selection immediately when `RestartLimiter` allows it, and otherwise the
  player keeps the frame and retries once the limiter window elapses. Captures
  set healing off, so a degenerate run still exits 1 instead of silently
  swapping the attractor. `Player::render(context)` creates the
  `FrameRenderer` lazily, resizes it when the surface changed and returns
  tightly packed RGBA8 rows.
- **I7a renderer**: `FrameRenderer` gained `resize`, `overlay_scale` and an
  `Attachment` bundle that recreates only the attachment textures and readback
  buffer. `with_sample_count(1)` now renders straight into the resolve texture
  without a resolve target, which closes the I6 deferred wgpu rejection and
  prepares the WebGL2 path in I9. `Player::resize` only records the new size;
  the renderer catches up on the next `render`, so the call needs no GPU
  context.
- **I7a C ABI**: `attractor-ffi` exports create/destroy/tick/render/resize/
  restart/width/height/last_error over an opaque handle. `AttractorPlayerOptions`
  carries the seed (zero means clock-based), size, overlay scale and a noop
  backend flag for CI. Errors come back as status codes with a
  NUL-terminated message from `attractor_player_last_error`; a null handle
  maps to `ATTRACTOR_PLAYER_INVALID_ARGUMENT`. `include/attractor.h` is
  hand-maintained (ruling in the design doc) and `tests/header.rs` fails when
  the header and the `#[unsafe(no_mangle)]` exports drift apart.
- **I7a verification**: the seeded CLI snapshot is unchanged bit for bit,
  which proves the player extraction preserved the geometry. On an Apple M4
  Pro the CLI capture produced 2400x1800, 872 368 lit pixels — the same number
  as the I6 manual run. The parity test and the player resize path were
  additionally exercised on Metal through temporary `cargo run` examples
  (deleted afterwards): the player frame was byte-identical to the CLI frame
  at 640x480, and a 320x240 frame resized to 160x120 still drew ink. The
  committed GPU tests skip with a printed reason when no adapter is available.
  Locally they do run: `.claude/settings.local.json` takes bare `cargo test …`
  out of the agent sandbox (see the sandbox note), and a full
  `cargo test --workspace` exercised the GPU renders, the player/CLI parity
  test and the native FFI render on Metal. CI has no GPU, so its deterministic
  gate remains the snapshot plus the noop-player tests.
- **I7a deferred**: rendering goes through an offscreen readback; drawing into
  the host's `CAMetalLayer` drawable is I7b work, as is the Swift view.
  `attractor_player_create` returns null on failure without an error handle,
  because there is no player to carry a message yet; the reason goes to the
  `log` crate. The parity test repeats the CLI's construction instead of
  calling `prepare`, so it stays an independent check rather than a tautology;
  if the CLI defaults change, the test must change with them.
- **Deferred cleanup (2026-11-12)**: the GPU-dependent gap is gone — the
  project sandbox config excludes `cargo test` and `cargo test …`, so the
  suite runs bare and reaches Metal; `cargo test --workspace` is green with
  the render, text, parity and FFI tests actually executing. The remaining
  minor debts were closed in the same pass: formula domain edges, the
  non-finite overlay spelling, the `--frames` cap, the fixture README run and
  the overlay pixel test. Two unused accessors (`FrameRenderer::overlay_scale`
  and `PreparedRun::overlay_scale`) were removed.
- **I7b**: the classic `.saver` now runs on the Rust player. `attractor-gpu`
  gained `SurfaceTarget` (a swap chain created from a `CAMetalLayer` through
  `SurfaceTargetUnsafe::CoreAnimationLayer`) and
  `FrameRenderer::for_surface`/`render_to_view` (the same multisample ribbon
  and glyphon overlay pipelines drawing into an external texture view);
  `Player::render_to_surface` acquires, draws and presents. `attractor-ffi`
  gained `attractor_player_attach_layer` and `attractor_player_draw`, and the
  hand-maintained header and its drift test were extended. The Swift host is
  reduced to `MetalSurfaceView` (a layer-backed `NSView`) and
  `AttractorPlayerHost` (a 60 Hz run-loop timer, resize, create/destroy); no
  Swift code imports SceneKit or the catalog any more. The overlay is drawn by
  the engine, as D8 requires.
- **I7b surface verification**: a temporary example (deleted afterwards)
  created a real `CAMetalLayer` through the Objective-C runtime, attached it
  and presented 120 frames through the C ABI on Metal without an error. This
  covers surface creation, swap-chain configuration, 4x MSAA resolve into the
  drawable and presentation; it is not a substitute for the visual checklist.
- **I7b portability**: `wgpu::SurfaceTargetUnsafe::CoreAnimationLayer` exists
  only on Apple targets, so the Metal constructor, its configuration helper
  and the `SurfaceSource` variant are gated on `macos`/`ios`. `SurfaceTarget`
  itself, `acquire`/`resize`/`recreate` and
  `attractor_player_attach_layer` keep their signatures on every platform; the
  layer entry point returns `ATTRACTOR_PLAYER_ERROR` off Apple, and
  `tests/c_api.rs` pins that with a non-Apple-only test. Verified locally with
  `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu
  -- -D warnings` after installing that Rust std.
- **I7b wake-up parity**: `Player::tick` now clamps a finite interval to the
  scheduler's `maximum_frame_interval` before it reaches the camera and the
  colour gradient, exactly as the Swift `SimulationScene` did. A 60 s sleep
  jump therefore draws the same frame as a normal frame at the 0.1 s bound;
  `a_long_stall_is_clamped_like_the_swift_scene` pins it. The deferred-restart
  retry uses the same clamped interval.
- **I7b build**: `Host/project.yml` keeps only the classic `.saver` target
  (the `.appex`, its container app and `AppSources/` are deleted, D10). A
  pre-build phase compiles `attractor-ffi` for every `ARCHS` entry
  (`arm64` → `aarch64-apple-darwin`, `x86_64` → `x86_64-apple-darwin`) and
  `lipo`s the results into `libattractor_ffi.a`; the target links
  `-lattractor_ffi -liconv` plus the frameworks reported by
  `cargo rustc -p attractor-ffi --release -- --print native-static-libs`
  (Metal, QuartzCore, CoreGraphics, CoreFoundation, Foundation). The bridging
  header `Sources/AttractorBridge.h` includes the crate's `attractor.h`, so
  the host and the later Android bridge share one declaration file. The local
  Release check used `ARCHS=arm64`; a universal build requires the
  `x86_64-apple-darwin` Rust standard library, and the script says so.
- **I7b ruling (SceneKit removal)**: "SceneKit renderer removed" is read as
  removed from the host path, not deleted from the repository. `Render/` and
  `Demo/` stay until I12 because the I7b verification requires `Demo` to build
  and the visual reference is the Swift path pinned at `ef3d61b`; deleting
  them now would remove the oracle before the migration is accepted. Cost if
  wrong: the repository carries the SceneKit package for a few more
  iterations.
- **I7b manual checklist: preview and fullscreen verified 2026-11-12** on
  macOS 26.6.2 (M4 Pro) with the Release arm64 bundle installed into
  `~/Library/Screen Savers/`. The saver starts from System Settings → Screen
  Saver and from `open -b com.apple.ScreenSaver.Engine --args -module
  AttractorSaver`; attractors render and the overlay shows the live formula.
  Do not exec `ScreenSaverEngine` directly: macOS 26 kills it with
  `SIGKILL (Code Signature Invalid)` / launch-constraint violation before the
  module loads, which is not a saver failure. Multi-display (one player per
  screen) is still unverified — no second display was attached; wake/sleep and
  the health-monitor restart stay covered by the deterministic tests
  (`a_long_stall_is_clamped_like_the_swift_scene`, `RestartLimiter`). The
  iteration is "done (manual: preview + fullscreen)" until the multi-display
  pass is recorded here.
- **I7b deferred**: `SurfaceTarget` currently has only the Metal constructor;
  I8/I10 add constructors for a raw Windows/Linux window handle to the same
  type, keeping one presentation path. Notarization stays deferred to I12
  (D11).
- **I8**: the Windows `.scr` exists. `attractor-windows-saver` is a lib plus a
  binary; the lib holds the platform-neutral pieces (`args`, `settings`,
  `selection`, `input`, `text`, `self_test`) so they are unit-tested on every
  OS, and `windows::*` is the Win32 shell behind `#[cfg(windows)]`. The binary
  is the future `StrangeAttractor.scr`: Windows invokes it with `/s`, `/c`,
  `/c:HWND` or `/p HWND`, an empty command line opens the settings, and
  `--self-test` (with `--noop`, `--width`, `--height`) renders a frame without
  a desktop. `attractor-gpu` gained `SurfaceTarget::from_window_handle`, which
  wraps the `HWND` through `wgpu::rwh`, and `GpuError::InvalidWindowHandle`;
  presentation is the same swap-chain path as macOS.
- **I8 shell**: one borderless topmost `WS_POPUP` window per monitor from
  `EnumDisplayMonitors`, one `Player` and one swap chain per window, and a
  single 16 ms `WM_TIMER` that ticks every display with its measured `dt`.
  The process is per-monitor DPI aware, so monitor rectangles and surfaces are
  physical pixels; the overlay scale is the window DPI over 96. A key,
  mouse button, wheel notch or pointer motion ends the session; pointer motion
  is ignored for 250 ms after the start, which drops the `WM_MOUSEMOVE` left
  over from the click that launched the saver. `/p` creates a `WS_CHILD` window
  inside the control panel's preview control, refuses to exit on input,
  resizes with `WM_SIZE` and quits when the parent window disappears.
  `WM_DISPLAYCHANGE` ends the session instead of presenting into stale
  surfaces. The cursor is hidden for the session and restored on exit. The
  binary carries `#![cfg_attr(windows, windows_subsystem = "windows")]`, so
  Windows does not give the saver a console window; the self-test report still
  travels through the inherited pipe, which is why `cargo run … --self-test`
  shows it in CI.
- **I8 ruling (raw Win32, no winit)**: the design sketch said "Win32/winit"; the
  shell uses `windows-sys` directly. One borderless window per monitor, a
  timer loop, `/p` inside a foreign `HWND` and `/s`'s `SetForegroundWindow` /
  `ShowCursor` are all a few Win32 calls, while winit would add an event-loop
  abstraction no other shell needs. The design doc's I8 section records the
  ruling.
- **I8 settings**: `%APPDATA%\StrangeAttractor\settings.json`, JSON with one
  key (`attractor`) today. `/c` is a native window with a label, a combo box
  (catalog order plus "Random each time") and OK/Cancel; it writes the file
  atomically (temporary file plus rename) and shows a message box if the write
  fails. Loads treat a missing file as defaults, log and default a corrupt
  file, and fall back to random when the id is empty or not in the catalog, so
  the saver never fails to start because of its settings. Unknown keys are
  ignored for forward compatibility.
- **I8 self-test**: seeded (42), 5 000 warm-up steps, 60 frames at 640x480,
  reported with adapter, backend and lit pixels. On the M4 Pro the real
  adapter reported `Apple M4 Pro`, `Metal`, 6 953 lit pixels; the noop backend
  built the pipelines and reported 0 ink, as designed. With no adapter the
  test prints a skip and exits 0 (the convention the GPU-backed tests follow);
  CI therefore runs both the noop gate and the adapter gate, and the second
  draws WARP pixels on `windows-latest`.
- **I8 CI**: a new `windows` job runs `cargo fmt --check`, clippy for
  `attractor-windows-saver` and `attractor-gpu` on the Windows target
  (including the D3D12 surface code), the crate's tests, then both self-test
  modes. The ubuntu `rust` job is unchanged. Locally the Win32 code was
  verified with the same clippy command against `x86_64-pc-windows-gnu` and
  the shell was exercised through `--self-test`; the first real Windows run was
  CI run **43316581080** (push of `195d475`, 2026-11-12T22:05Z), which completed
  successfully in 6m36s with the `windows` job green.
- **I8 deferred**: the settings window exposes only the attractor choice;
  signing, an installer and the `.scr` packaging step belong to I12, and the
  crate README documents the build-and-rename recipe and SmartScreen note. A
  `.scr` whose window loses the foreground would miss input (no low-level
  hook); Windows normally grants the saver foreground. Multi-monitor and
  preview remain on the manual checklist until the Windows 11 pass.

- **I9**: the web demo exists. `attractor-wasm` is the browser shell over
  `attractor-player`; `web/` is the static page (full-screen canvas, catalog
  picker with a search box, seed field, URL-backed state), with a Playwright
  suite and a GitHub Pages workflow. The native half of the crate —
  `options`, `selection`, `catalog_view` — is plain Rust with unit tests, so
  the URL contract and the seeded selection are covered without a browser;
  the wasm-only `web` module owns the canvas surface, the player and the page
  API.
- **I9 surface**: `attractor-gpu` gained `SurfaceTarget::create_web`, which
  creates the instance, the canvas surface, the adapter, the device and the
  configured swap chain in that order and returns the pair. The adapter is
  requested with `compatible_surface`, because the WebGL2 backend binds the
  adapter to the canvas context (the order the I0.5 spike verified in Chrome).
  `GpuContext` is assembled through the crate-private `from_parts`; `pollster`
  is now a non-wasm dependency and wgpu's `webgl` feature is wasm-only, so the
  desktop build is unchanged. `GpuContext::backend_label` and
  `supports_multisampling` expose the two facts the page needs.
- **I9 features**: `attractor-player` gained a `text` feature that forwards to
  `attractor-gpu/text`; the wasm crate depends on the player with
  `default-features = false`, so glyphon/cosmic-text are not in the bundle and
  the overlay is DOM text (D8). WebGL2 cannot resolve multisampled targets, so
  the demo passes `sample_count = 1` when the backend is GL.
- **I9 URL contract**: `?attractor=<id>&seed=<u64>&backend=webgl2`. Parsing
  is tolerant: a malformed seed or backend and unknown keys are ignored, and
  a missing or unknown attractor means "random". On first load the page
  invents a seed and writes it into the address bar, so any frame can be
  shared. A named attractor samples its parameters, hyperparameters and
  initial state from the declared ranges with the seed — the randomizer, not
  the midpoint debug configuration, because several attractors are degenerate
  exactly at their midpoints.
- **I9 build**: `web/build.sh` builds `attractor-wasm` for
  `wasm32-unknown-unknown` and runs `wasm-bindgen --target web` into
  git-ignored `web/pkg`; `wasm-opt -Oz` runs when the binary is present. The
  CI `web` job installs the wasm target, wasm-bindgen 0.2.130 (matching the
  crate) and binaryen, lints the wasm target with clippy, builds the bundle
  and runs Playwright.
- **I9 browser tests**: Playwright waits for frames, asserts a non-blank
  canvas (an element screenshot decoded with pngjs), asserts two screenshots
  700 ms apart differ (the camera animates), checks the overlay text and
  asserts a clean console (no `console.error`, no page errors). One test
  forces `?backend=webgl2` and asserts the reported backend — the ADR-1
  requirement; the default-URL test accepts `webgpu` or `webgl2`, because a
  headless runner without a working WebGPU adapter legitimately falls back.
- **I9 review pass**: the pixel assertions hide the control panel and the DOM
  overlay (`visibility: hidden`) before taking the canvas screenshot, because
  an element screenshot is a clip of the composited page and the translucent
  toolbar alone would satisfy the brightness threshold. The Pages path filter
  now includes the embedded catalog
  (`Core/Sources/AttractorCore/Resources/attractors.json`), the `web` CI job
  asks its toolchain for `clippy`, and a failing `wasm-opt` pass logs a
  warning instead of failing the build (the bundle is valid without it).
- **I9 deferred (minor, from review)**: the `github` reporter writes no HTML
  report and `test-results/` is empty without a trace configured, so a red
  browser test uploads an empty artifact; and an unknown `?attractor=` id
  leaves the picker blank while Rust correctly runs a random attractor. Both
  are worth a follow-up, neither affects the green path.
- **I9 Pages**: `.github/workflows/pages.yml` builds the bundle and publishes
  only `index.html`, `styles.css`, `app.js` and `pkg/`. The repository's Pages
  source is switched to "GitHub Actions" (done by the owner on 2026-11-13).
  It triggers on `main` (a `workflow_dispatch` from `rust-port` can publish
  earlier), so it never runs on the working branch's pushes; the URL goes
  live on the first `main` push or dispatch.
- **I9 local verification**: fmt, clippy for the workspace and for the wasm
  target, `cargo test --workspace` (run bare, so the Metal-backed tests
  executed), the release wasm build, the wasm-bindgen bundle, a Node
  `WebAssembly.compile` check of the exported functions (`start`,
  `catalog_json`, the `demo_*` accessors) and a static-server fetch of every
  asset. The Playwright suite could not run inside the agent sandbox:
  Chromium aborts with `bootstrap_check_in … Permission denied (1100)` because
  Seatbelt denies the Mach-port rendezvous. CI is the first real browser run;
  the manual Chrome/Safari/Firefox checklist remains with the reviewer.
- **I9 first browser run (CI 43500194026, `7dd7096`)**: `rust`, `windows` and
  the bundle build are green; the forced-WebGL2 test failed at
  `waitForFrames` with a bare 30 s timeout, and the automatic-backend test
  passed, so the page, the bundle and the frame loop work in headless
  Chromium. A timeout alone cannot tell "the runner has no WebGL2" from "the
  adapter refused the canvas" from "the first frame raised", so `app.js` now
  publishes `stage` (`module` → `wasm-ready` → `starting` → `running` →
  `failed`) and `error` on `window.__attractorDemo`, and the failure message
  carries that state, a `webgl2`/`webgpu` probe of the page and the collected
  console errors. The successful automatic-backend test also prints the
  backend it picked, because that fact exists only in a green run.
- **I9 WebGL2 device fix (CI 43501835180, `63d1cc0`)**: the instrumented run
  named the cause — the page reached `starting` and `start()` returned
  `cannot create a GPU device: Limit 'max_compute_workgroups_per_dimension'
  value 65535 is better than allowed 0`. WebGL2 has no compute support, so the
  GL adapter reports zero compute limits while `DeviceDescriptor::default()`
  asks for the desktop defaults; the browser now requests `adapter.limits()`
  instead. `--use-angle=swiftshader` does provide WebGL2 in the headless
  runner (the probe reported `WebGL 2.0 (OpenGL ES 3.0 Chromium)`), so the
  test was exercising the fallback, not a missing platform feature.
- **I9 CI green (43502605634, `d47538c`)**: all six jobs passed — the browser
  suite reports `2 passed (6.4s)`, covering the forced WebGL2 path and the
  automatic backend, and `rust`, `windows`, `Core tests`, `Render and Demo
  build` and `Catalog validation` are unchanged. The first WebGL2 rendering
  of the page therefore happens on a runner without a GPU.
- **I10**: the Linux shell exists. `attractor-linux-saver` is the xscreensaver
  hack (`-root`, `-window-id N`) and the fullscreen runner (`--fullscreen`,
  plus the no-argument default); `shells/linux/` carries the `.desktop` file,
  the xscreensaver entry, the `swayidle`/`hypridle` recipes, the settings
  location and the manual checklist. The platform-neutral pieces the Windows
  shell already owned — settings, attractor selection, input policy and the
  offscreen self-test — moved into the new `attractor-shell` crate, which both
  shells depend on; the Windows crate re-exports the same public names, so its
  behaviour and tests are unchanged.
- **I10 surface**: `attractor-gpu` gained
  `SurfaceTarget::from_raw_handles` (Linux only), and `SurfaceSource` a
  `RawHandle` variant that carries `raw-window-handle` values into wgpu. The
  xscreensaver mode passes the Xlib window and `Display*` it got from `x11-dl`
  and uses the window's own visual id; the runner passes what winit reports
  (Xlib/Xcb on X11, Wayland on Wayland). Presentation still goes through the
  single wgpu swap-chain path.
- **I10 backends**: the xscreensaver hack uses raw Xlib through `x11-dl`,
  loaded at runtime, because xscreensaver hands the hack a foreign window and
  expects the process to leave when that window is destroyed or unmapped;
  winit has no embedding API. The fullscreen runner uses winit 0.30, which
  covers both X11 and Wayland and gives the plugin access to monitors and
  input. Both share `session::Display`, the per-surface player wrapper, and
  the `attractor-shell` input policy (pointer motion is ignored for 250 ms
  after the first frame).
- **I10 rulings**: `-root` does not draw into the X root window — Vulkan
  presentation into the root is unreliable — so the shell creates an
  override-redirect window covering the screen; the visible result matches
  xscreensaver and the window disappears with the process. The settings file
  follows XDG: an absolute `XDG_CONFIG_HOME` wins, otherwise `~/.config`, and
  the file lives in `strange-attractor/settings.json` (Windows keeps
  `%APPDATA%\StrangeAttractor\settings.json`); the rule is a pure function
  tested on every OS. Linux has no settings dialog in this iteration; the JSON
  is documented and hand-edited.
- **I10 verification (local)**: fmt, workspace clippy and `cargo test
  --workspace` are green on macOS (Metal); the Windows cross-clippy is green
  after the shared-crate move; Linux-target clippy for
  `attractor-linux-saver` and `attractor-gpu` is green; the noop self-test
  validates the pipelines and the Metal self-test reports `Apple M4 Pro` and
  6 953 lit pixels — the same number as the Windows crate's self-test, which
  is expected because both run the same seeded player. The `attractor-gpu`
  Linux unit test (zero-size rejection before the handles are touched) runs in
  the ubuntu `rust` job. The CI `linux` job is the first run of the X11 and
  winit presentation paths; the job fails rather than skipping when no
  adapter is found. Native Wayland and xscreensaver's `-window-id` preview
  remain on the manual checklist.
- **I10 CI green (43507189418, `be48758`)**: all seven jobs passed on the
  first run. The `linux` job rendered the adapter self-test through
  `llvmpipe (LLVM 21.1.8, 256 bits)`, backend Vulkan, 6 763 lit pixels at
  640x480; the xscreensaver `-root` mode and the winit `--fullscreen` runner
  each presented 30 frames under Xvfb; the CLI captured a 327 996-byte PNG
  through the same software renderer (artifact `linux-screenshot`), and the
  downloaded artifact shows the Lorenz attractor and its overlay rather than
  an empty frame. The
  ubuntu `rust` job also ran the new `attractor-gpu` raw-handle test, and the
  `windows` job stayed green after the shared-crate move.
- **I10 deferred**: no settings UI on Linux (hand-edited JSON); the
  `.desktop` file is not installed system-wide by the build; packaging
  (tarball/AUR/deb) belongs to I12; `-window-id` is covered by the manual
  checklist rather than by CI because it needs a second X client to own the
  window. The runner creates one fullscreen window per monitor; a Wayland
  compositor that rejects several fullscreen surfaces falls back to whatever
  winit maps, and the manual pass records how each compositor behaves.
- **I10 review (`be48758`)**: an independent read-only review found no
  Critical or Important defects; the raw-handle constructor, the shared
  crate move and the CI job were all accepted as correct. Seven Minor
  findings are deferred; none blocks use:
  1. `-root` key exit: an override-redirect window is never focused, so on
     a desktop that sets focus explicitly a key press without pointer
     motion does not quit. Under xscreensaver the daemon handles input, and
     a click or pointer motion still ends the session, so only the README's
     "any key" wording overstates the standalone case. A keyboard grab
     would make it literal; it is a behavioural change that needs the
     manual pass.
  2. Xlib installs no error handler, so after a clean `DestroyNotify` the
     `XUndefineCursor` on xscreensaver's already destroyed preview window
     reaches the default handler: the saver exits 1 with an X-error trace.
  3. `XSelectInput` runs after GPU and pipeline creation, so input arriving
     during those seconds is lost.
  4. `ScaleFactorChanged` reconfigures with the stale `inner_size()`; the
     following `Resized` event corrects it a frame later.
  5. A surface that stays lost is retried silently at 60 Hz instead of
     ending the session (a black screen, like a blanked saver).
  6. Unknown dash flags are a usage error (exit 2, a black hack under
     xscreensaver); `-foreground`/`-background` are the flags a future
     xscreensaver is most likely to pass, and ignoring unknown ones would
     be more robust for an embedded hack.
  7. With `-window-id` on a non-default X screen the display handle names
     the default screen of `$DISPLAY` (unreachable on today's
     single-screen setups).
  The shared-crate move also shrank `attractor-windows-saver`'s re-exports
  (`SelfTestBackend`, `SelfTestError`, `SelfTestReport`, the `input`
  module); every crate is `publish = false` and nothing referenced the old
  paths, so this is drift rather than breakage.
- **I11**: the Android live wallpaper exists under `rust/shells/android/`
  (Kotlin `WallpaperService`, picker activity, preferences, Gradle project)
  and its JNI bridge lives in `attractor-ffi`. The engine gives every visible
  surface one rendering thread; the thread calls `NativeBridge.create` with
  the `Surface`, draws at 60 Hz, applies size changes and destroys the player
  before the surface goes away. The bridge owns six entry points (`create`,
  `render`, `resize`, `destroy`, `lastError`, `catalog`); `tests/android_bridge.rs`
  reads the Kotlin file and the Rust module and fails when the two name lists
  drift, and the platform-neutral half (`android_support`: catalog JSON, the
  id→settings rule, the frame interval, the seed rule) has host unit tests.
- **I11 surface**: `attractor-gpu` gained `SurfaceTarget::create_android`,
  which creates the instance, surface, adapter, device and swap chain in one
  call — like the browser bootstrap, because the GLES backend binds the
  adapter to the window. The Kotlin side passes `ANativeWindow_fromSurface`,
  so there is no EGL code in the shell (ADR-2). The device asks for
  `adapter.limits()` (a GLES adapter reports smaller limits than the desktop
  defaults), and a GL backend renders single-sample, because GLES cannot
  resolve multisampled targets on every driver.
- **I11 rulings**: the JNI bridge stays in `attractor-ffi`, which the design
  already names as its home, rather than in a new crate. The settings are
  `SharedPreferences` with the same `attractor` key and the same
  "empty means random" rule as the desktop files; an unknown id falls back to
  a random attractor. A change applies when the wallpaper engine restarts
  (re-apply or lock/unlock), which avoids adding a broadcast path into a
  running engine this iteration. minSdk is 26, targetSdk and
  compileSdk are 35, and the build produces `arm64-v8a` and `x86_64`.
- **I11 verification (local)**: fmt, workspace clippy and `cargo test
  --workspace` are green on macOS; `cargo clippy -p attractor-ffi
  -p attractor-gpu --target aarch64-linux-android --all-targets` is green;
  `gradle assembleDebug lint` built an 805 KB debug APK and lint reported
  "No issues found." The APK in that local run has no `jniLibs`, because the
  NDK is not installed in the agent sandbox; CI installs it, builds the
  libraries first and uploads a complete APK. The emulator/phone pass is the
  remaining gate.
- **I11 CI green (43514690865, `d3a51d5`)**: all eight jobs passed. The
  `android` job installed NDK 27.2, built `libattractor_ffi.so` for
  `arm64-v8a` and `x86_64`, linted the Android target, ran
  `assembleDebug lint` and uploaded a 23 MB APK; the artifact contains both
  libraries, and `llvm-nm -D` on the arm64 one lists exactly the six
  `Java_com_xekchansky_strangeattractor_NativeBridge_*` symbols. The first
  run of the job failed inside `android-actions/setup-android@v3`, which
  installs the legacy `tools` package that the current SDK repository no
  longer publishes; the job now uses the SDK the ubuntu image ships and
  calls its `sdkmanager` directly.
- **I11 review (`48845e3`)**: an independent read-only review found no
  Critical defects. One Important — the NDK path depended on
  `ANDROID_SDK_ROOT` rather than the guaranteed `ANDROID_HOME` — was fixed
  in `05b0a36`. Six Minor findings are deferred; none blocks use:
  1. a panic inside the player would unwind through `extern "system"`;
     `panic = "abort"` for the cdylib profile or `catch_unwind` would make
     it well defined;
  2. `STATUS_ERROR`/`STATUS_UNHEALTHY` log a warning every frame, so a
     persistently failing swap chain becomes 60 Hz log spam;
  3. if the 2 s join in `stopRendering` times out, a prompt visibility
     change can start a second player on the same window (each thread holds
     its own window reference, so this is visual corruption, not a refcount
     bug);
  4. `CREATE_ERROR` is process-global, so two engines (preview and main
     surface) share the create error text;
  5. a failed catalog fetch leaves the picker with only "Random" and no
     message;
  6. the "handle must be live" contract is only a SAFETY comment; Kotlin
     honours it, but a debug-only generation counter would harden it.
- **I11 deferred**: release signing, versioning and Play/F-Droid packaging
  belong to I12; the picker does not push a new choice into a running engine;
  there is no on-device self-test mode (the shared player's self-test is
  exercised by the desktop shells, and Android is covered by the build and
  the manual pass). The wallpaper targets the live-wallpaper idiom, so the
  formula overlay is drawn by the engine exactly as on the desktops.
- **Sandbox note**: in the agent sandbox `~/.cargo` is not writable, so local
  builds point `CARGO_HOME` at the git-ignored `rust/.toolchain/cargo` and
  `RUSTUP_HOME` at `rust/.toolchain/rustup`. CI and a normal developer shell
  use the default locations; nothing in the repository depends on this. The
  project's git-ignored `.claude/settings.local.json` also lists `cargo run *`,
  `cargo test` and `cargo test *` in `sandbox.excludedCommands`, so a bare
  command reaches Metal. The exclusion matches the command as written: a
  `CARGO_HOME=…` prefix, a pipe or a chain around a GPU run keeps it inside
  the sandbox, where `cargo test` still passes but skips the adapter tests.
  Likewise, `swift test` inside the agent sandbox needs `--disable-sandbox`,
  because SwiftPM cannot start its own nested sandbox there. When that flag is
  unavailable, the same fixture generators can be driven by compiling the
  `Core/Sources` files with `swiftc` directly (excluding
  `AttractorCatalog.swift`, whose `Bundle.module` is SwiftPM-generated, and
  loading `attractors.json` by path); the values are identical because both
  paths call the same oracle functions.
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
  reach Metal. The git-ignored `.claude/settings.local.json` lists
  `cargo run *` under `sandbox.excludedCommands` and allows the crates.io
  domains, so an agent can reproduce the headless capture. Pipes, `&&` chains
  and a leading `time`/`export` drop the exclusion: the whole call must be a
  bare `cargo run …`. The same sandbox denies creating a `.vscode` directory
  inside the repository, so a crate whose `.crate` archive contains one (for
  example `bit-vec`) fails to unpack into `rust/.toolchain/cargo`. Unpack it
  manually, skipping that entry, and write the `{"v":1}` `.cargo-ok` marker
  afterwards; CI does not hit this.
- The toolchain is plain stable Rust (edition 2024); no toolchain version is
  pinned yet. I0–I6 were verified locally with rustc 1.99.1 (stable). A local
  rustup home lives in `rust/.toolchain` (git-ignored) because the Homebrew
  rustc ships only the host standard library. The dependency set is pinned:
  wgpu 30 and glyphon 0.12 (I6).
- wgpu 30 restructured its public API, and the 24–26 documentation snippets do
  not compile. The working reference in this repository is now
  `crates/attractor-gpu/src/{ribbon,frame}.rs`; the I0.5 spike that used to
  serve this role was deleted in I6.
