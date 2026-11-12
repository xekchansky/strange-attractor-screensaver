# Rust port — status

Design and iteration plan:
[`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](../docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md).

Protocol for a fresh session: read the design, re-run the previous
iteration's verification command, implement exactly one iteration, update this
file, and commit atomically.

## Current state

- Branch: `rust-port`
- Last completed iteration: **I7b — macOS host migration** (2026-11-12)
- Last completed change: **I7b — macOS host migration** (2026-11-12)
- Next iteration: **I8 — Windows `.scr`**
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
- [ ] I8 — Windows `.scr`
- [ ] I9 — web demo on GitHub Pages
- [ ] I10 — Linux
- [ ] I11 — Android live wallpaper
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
