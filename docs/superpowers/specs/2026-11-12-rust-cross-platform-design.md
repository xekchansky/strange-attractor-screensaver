# Cross-platform Strange Attractor Screensaver — Design

- **Status:** draft, awaiting owner approval
- **Branch:** `rust-port`
- **Date:** 2026-11-12
- **Owner:** repository author

## 1. Intent

Make the screensaver reachable by a stranger: someone finds the repository
(by search or by chance), immediately sees the attractors running in a
browser, and installs the saver on their own system — macOS, Windows, Linux
or Android — without building anything from source.

The visible product is the same everywhere: a chaotic trajectory integrated
in real time, drawn as a colored ribbon that shows the direction of motion,
orbited by the camera, with the formulas and live position in a corner
overlay. Today that lives in Swift + SceneKit and only runs on macOS.

## 2. Goals, non-goals, anti-goals

**Goals**

- One simulation + rendering engine, one catalog, one visual identity on
  every platform. Ports must not become independent forks.
- A browser demo on GitHub Pages as the discovery funnel: try before install.
- Installable artifacts per OS: `.saver`/`.dmg` (macOS), `.scr` (Windows),
  runnable binary + xscreensaver/fullscreen integration (Linux), `.apk`
  (Android).
- Every iteration is independently verifiable: a test suite, a headless
  render, or a documented manual checklist.
- The existing numerical contracts survive the port: RK4, catalog-driven
  parameters, deterministic seeding.

**Non-goals**

- iOS/iPadOS, tvOS, watchOS, webOS, browser extensions.
- A store-quality settings UI on every platform. Settings can start as a
  config file / environment variables and grow later.
- Cloud sync, telemetry, accounts, auto-update infrastructure.
- Keeping the Swift engine alive long-term. Swift remains only as a thin
  AppKit host for the macOS `.saver` bundle.

**Anti-goals**

- Two engines (Swift and Rust) maintained in parallel after migration.
- Requiring a compiler or a terminal from a non-technical user.
- Per-platform copies of the catalog, the formula parser or the solver.
- A port that looks different from the current screensaver.

## 3. Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | Rewrite engine and renderer in **Rust** | One implementation compiles to macOS, Windows, Linux, Android and WebAssembly; strong types and `cargo test` match the repo's culture; `serde` reads the existing catalog unchanged. |
| D2 | Render with **wgpu** (Metal / D3D12 / Vulkan / GLES backends) | One pipeline and one shader language (WGSL) across desktop and Android; headless rendering makes the renderer testable in CI. |
| D3 | **One player library** shared by all shells; shells stay thin | Prevents the drift that two renderers would guarantee. |
| D4 | **macOS migrates to the Rust engine**; Swift is reduced to an AppKit view + `CAMetalLayer` host, and to a test oracle during migration | Keeps the `.saver` plugin shape (Cocoa view) while removing the duplicated engine. |
| D5 | Work happens on the **`rust-port`** branch; `main` stays shippable | The Swift saver keeps working until the Rust host is manually validated. |
| D6 | The catalog stays at `Core/Sources/AttractorCore/Resources/attractors.json` during migration, read by Rust with `include_str!` | Single source of truth until the Swift package is retired; moved to `data/attractors.json` in I12. |
| D7 | Web demo is a static site on **GitHub Pages**, built from the same Rust engine via WebAssembly | Cheapest honest "try it now" path; no server. |
| D8 | Text overlay is rendered by the engine (native) and as a DOM overlay in the browser | Avoids four native text stacks; DOM text is accessible and crisp on the web. |
| D9 | Platforms ship **as they become ready**: macOS + web first, then Windows, Linux, Android as separate releases | Each iteration yields a verifiable artifact; no release blocked on the slowest platform. |
| D10 | Keep only the classic `.saver` host; drop the `.appex`/container-app variant | One macOS surface to migrate; the container path is not needed for distribution. |
| D11 | Notarization is **deferred to I12**; releases stay ad-hoc signed with the documented `xattr` note | No paid Apple Developer account/CI secrets are committed to now; the decision is re-opened before the first release. |

## 4. Architecture

```
rust/
  crates/
    attractor-core    data model, catalog, formula parser, RK4, RNG,
                      scheduler, trace buffer, health monitor, engine
    attractor-view    camera math, trail geometry, color ramps, overlay text
    attractor-gpu     wgpu device/pipeline, ribbon rendering, text rendering
    attractor-player  orchestration: engine + view + gpu + surface, tick/render
    attractor-ffi     C ABI used by the macOS `.saver` and Android JNI
    attractor-cli     cross-platform demo, `--screenshot`, `--record`
    attractor-wasm    wasm-bindgen glue for the GitHub Pages demo
    attractor-windows-saver  `.scr` shell (Win32/winit, /s /c /p)
  STATUS.md           iteration checklist + current state
data/                 (I12) canonical attractors.json
web/                  static demo site
shells/macos/         Swift AppKit view hosting the player (existing Host/)
shells/linux/         xscreensaver hack + fullscreen runner + .desktop
shells/android/       Kotlin WallpaperService + JNI bridge
```

Dependency direction: `core` ← `view` ← `gpu` ← `player` ← shells.
`core` must stay pure Rust: no `wgpu`, no `winit`, no platform code.
`view` must stay GPU-free and deterministic.
`gpu` may depend on `wgpu` and `wgpu`-based text only.

Data flow per frame: `player.tick(dt)` advances the engine and the camera,
`view` builds positions/colors for the trail, `gpu` uploads and draws. The
health monitor can request a restart; the player re-rolls a selection.

## 5. Numerical fidelity contract

- **SplitMix64 must be bit-identical** to the Swift implementation. Its
  sequence is platform-independent, so Rust tests compare exact `u64` values.
- **Formula evaluation** is compared against golden fixtures produced by the
  Swift parser: same expression, same variable values, tolerance 1e-12.
  `sin`/`cos`/`pow` come from different libm builds, so bit equality is not
  promised; the tolerance absorbs ULP differences.
- **Trajectories** are compared over short horizons (≤ 1000 RK4 steps) with a
  per-step tolerance of 1e-9 relative. Long horizons are chaotic: any ULP
  difference grows exponentially, so long runs are validated by invariants
  instead — finite states, boundedness, health-monitor behaviour for fixed
  points, limit cycles and divergent systems.
- **End-to-end determinism**: a fixed seed produces the same selection,
  parameters, initial state and `N`-step trace in Rust as in Swift. This is
  the gate that proves the port, and it depends on I2+I3 being exact.
- Golden fixtures are generated once by a Swift test gated behind
  `ATTRACTOR_DUMP_FIXTURES=1` and committed under
  `rust/crates/attractor-core/tests/fixtures/`. The generator command is
  documented next to the fixtures and re-run only when the contract changes.

**Swift oracle.** The oracle is the Swift 6.2 toolchain on the `macos-26` CI
runner, at the commit recorded in `rust/STATUS.md` when I2 starts. A seed's
selection does not depend on SplitMix64 alone: Swift's range sampling
(`Double.random(in:using:)`, `Int.random(in:)`, `ClosedRange` vs `Range`,
`randomElement`) belongs to the standard library. I2–I4 must port those
sampling algorithms exactly and cover `Randomizer.select` (not just formulas
and trajectories) with fixtures. After I12, `Core/` is retired: the fixtures
become immutable and the oracle is preserved as an archived git tag/branch.
Regenerating fixtures requires checking out that tag; the Rust tests never
depend on a live Swift toolchain.

**Visual acceptance.** "Identical" means: same attractor, same parameters and
initial state for a fixed seed; same camera projection/FOV/up vector, orbit
constants (`elevation = π/5`, `framingFactor = 2.1`, `smoothing = 0.06`,
framing refresh every 6 frames, random azimuth at start), same color pipeline
(sRGB surface + SceneKit's gamma handling), same four color modes
(`trail`, `age`, `depth`, `spatial`) and the same world-space ribbon radius
derived from the trail's bounding radius — not a fixed screen-space width.
Acceptance is manual side-by-side against the last Swift build (commit pinned
in `rust/STATUS.md`) plus archived reference screenshots. CI runs a loose
perceptual diff only as a regression signal, never as the acceptance bar.

## 6. Platform reality (the install story)

| OS | Mechanism | Artifact | Honest caveat |
|----|-----------|----------|----------------|
| macOS | ScreenSaver `.saver` bundle (AppKit view) | ad-hoc signed `.saver.zip` | Already working; notarization deferred to I12; migration must be visually identical. |
| Windows | `.scr` executable: `/s`, `/c`, `/p HWND` | `.scr` (+ optional installer) | Unsigned binaries trigger SmartScreen; signing certificate is a separate cost decision. |
| Linux (X11) | xscreensaver hack binary | tarball / AUR / `.deb` later | Wayland has no screensaver API; ship a fullscreen runner plus `swayidle`/`hypridle` recipes. |
| Android | live wallpaper (`WallpaperService` + EGL) | `.apk` (sideload; Play/F-Droid later) | The OS offers no "screensaver"; live wallpaper is the idiom. |
| Web | WebAssembly + canvas | GitHub Pages static site | Browser GPU support varies; the I0.5 spike decides WebGPU vs WebGL2. |

**Release cadence:** macOS + web ship first; Windows, Linux and Android each
ship as their own release when their iteration is green (D9).

**Support matrix (defaults, builder may narrow if CI cannot cover them):**
macOS 13+ universal (`arm64` + `x86_64`); Windows 10/11 x64 (ARM64 later);
Linux x86_64 with a glibc baseline on X11 and Wayland; Android
`arm64-v8a` plus `x86_64` for the emulator; browsers — current
Chrome/Edge/Safari/Firefox, WebGPU with an automatic WebGL2 fallback
(ADR-1 in I0.5).

**Manual verification.** Manual access exists but is infrequent: a friend's
Windows 11 PC, Debian and Ubuntu machines, and an Android phone. Therefore
CI and emulators are the acceptance gate for every iteration; manual checks
are scheduled together with that friend at milestone boundaries (after I8,
I10, I11 and before each release) using a checklist prepared in the iteration,
and recorded in `rust/STATUS.md`. An iteration can be "done (manual: pending)"
without blocking the next one; a release cannot.

**"Installable without building" scope:** prebuilt artifacts only — no
compiler for any platform. Terminal instructions are acceptable on Linux
(and APK sideload on Android); macOS and Windows must be double-click
installable. Building from source is never required.

## 7. Iterations

Each iteration is sized for one session, ends with a green verification
command, and updates `rust/STATUS.md` plus one atomic commit.
`S` ≈ under a day, `M` ≈ a few days, `L` ≈ a week or more.

### I0 — Workspace skeleton and CI (S)

**Goal:** a Rust workspace that builds and tests in CI with nothing else
touched.

**Deliverables:** `rust/Cargo.toml` workspace; empty crates with one smoke
test each; `rust/STATUS.md`; a `rust` job in `.github/workflows/ci.yml`
(`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace`).

**Verify:** `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`; CI green.

**Done when:** the Swift jobs are unchanged and still green; no files outside
`rust/`, `docs/` and CI are touched.

### I0.5 — Renderer / web spike, timeboxed (M)

**Goal:** retire the two biggest unknowns before the long port.

**Deliverables:** throwaway prototype drawing 10k colored ribbon segments
with wgpu; a wasm build served locally; `cargo check` for
`aarch64-linux-android` with the GLES backend. Findings and the WebGPU vs
WebGL2 decision recorded in this document as an ADR.

**Verify:** prototype runs on macOS; the wasm page renders in Chrome/Safari;
Android target compiles.

**Done when:** a short ADR answers: (a) web target — WebGPU, WebGL2, or both;
(b) Android renderer — wgpu GLES or raw EGL; (c) any dependency that will not
build everywhere.

#### I0.5 findings and ADR (2026-11-12)

The spike lives in `rust/spike/` (excluded from the workspace, deleted once
`attractor-gpu` has the real pipeline in I6). It integrates a Lorenz orbit,
builds a 20 002-vertex ribbon (10 000 segments, two vertices per point, hue
ramp per vertex) and draws it with one WGSL pipeline. The camera uses the
saver's constants: elevation `pi/5`, distance `radius * 2.1`.

**Status of the verification.** All checks are green: `cargo test` (14 tests:
orbit finiteness, ribbon topology, head presence, color variation, camera
math, sRGB clear-color encoding, the frame clock, WGSL validation through
`naga`, and a full pipeline build through wgpu's `noop` backend), the wasm
build, `cargo check --target aarch64-linux-android`, and both visual checks.
The native frame rendered on an Apple M4 Pro (`Metal, IntegratedGpu`) with
5.40% of the pixels lit; the wasm page rendered in Chrome, and two DevTools
screenshots taken 2.5 s apart differed, so the camera animates. Two defects
had to be fixed to get there: the "not blank" metric compared the linear
clear color with an sRGB-encoded readback and counted every pixel as lit, and
the web loop subtracted `Date.now()` (epoch milliseconds) from the rAF
timestamp, which made the `f32` camera angle stick and froze the picture. The
checks are repeatable by an agent through `rust/spike/web-check.sh`, listed
in the git-ignored `.claude/settings.local.json` under
`sandbox.excludedCommands`, because the macOS sandbox cannot start a browser
or reach Metal. Headless `--virtual-time-budget` does not drive rAF (three
frames, 17 ms observed), so animation is verified with a live DevTools
session rather than two virtual-time screenshots.

**ADR-1 — Web: WebGPU with an automatic WebGL2 fallback.**
Build one wasm binary with both the `webgpu` and `webgl` wgpu features and
let `wgpu::util::new_instance_with_webgpu_detection` choose: WebGPU when the
browser exposes a working `navigator.gpu` adapter, WebGL2 otherwise. Reasons:
WebGL2 is the compatibility floor for browsers and GPUs without WebGPU, and
the fallback costs no second renderer — it is the same WGSL pipeline compiled
by naga to GLSL. WebGPU is preferred when present because it removes the
WebGL2 limits and has lower driver overhead for future work (MSAA, bloom).
Constraint accepted: the production renderer must stay inside the WebGL2
subset — no compute passes, no storage buffers, one uniform buffer, vertex
and index buffers only. The ribbon pipeline already fits. The fallback also
means the shader must remain WGSL, not a WebGPU-only extension set.
Consequence for I9: the Playwright test must assert which backend it got and
run the WebGL2 path at least once (e.g. by disabling WebGPU).

**ADR-2 — Android: wgpu's GLES backend, not raw EGL.**
`cargo check --target aarch64-linux-android` succeeds with wgpu's default
`gles` + `vulkan` features and pulls `ndk-sys`, but needs no NDK link step.
The live wallpaper gives us an EGL surface from `WallpaperService`; the
native window behind it (`ANativeWindow_fromSurface`) is exactly the handle
wgpu's GLES backend can create an EGL surface from, so the player keeps one
renderer for every platform. Raw EGL would mean a second shader/layout path
duplicated on Android — the anti-goal. Fallback if surface creation from the
wallpaper window turns out to be unsupported: render offscreen in wgpu and
blit the texture to the EGL surface; the cost is one copy per frame, decided
in I11 on a real device.

**ADR-3 — Dependencies and versions.**
- `wgpu` 30.0.1 builds for macOS (Metal), `wasm32-unknown-unknown`
  (WebGPU + WebGL) and `aarch64-linux-android` (GLES + Vulkan). It is a large
  API break from the 24–26 line (`SurfaceTarget::Canvas`, `queue.present`,
  `CurrentSurfaceTexture`, `PipelineLayoutDescriptor::immediate_size`,
  `Option<&BindGroupLayout>` entries); the workspace must pin the major
  version and expect churn on upgrade.
- `wasm-bindgen` must match the `wasm-bindgen-cli` version exactly
  (0.2.130 here). I9 pins both (or uses `wasm-pack`) and runs `wasm-opt`;
  the spike's release bundle is 4.2 MB before shrinking.
- `bytemuck`, `pollster`, `png`, `env_logger`, `naga` build on all three
  targets. `pollster` is native-only in practice (browser code awaits the
  futures instead).
- wgpu's `noop` backend with `backend_options.noop.enable = true` accepts the
  full pipeline without a GPU. It is a good CI guard for descriptor mistakes
  before I6 adds real headless rendering.

### I1 — Data model and catalog (S)

**Goal:** the exact same catalog decoded and validated in Rust.

**Deliverables:** `MinMax`, `State`, `StateRange`, `Derivatives`,
`Parameter`, `SimulationHyperparameters`, `SimulationSettings`,
`AttractorDescriptor`, `TraceColorMode`; serde decoding; `AttractorCatalog`
with the same validation rules (range sanity, duplicate ids/parameters,
formula syntax, unsupported functions, undeclared variables). The last two
rules need the formula AST, its function table and the recursive-descent
parser, so those land here too — syntax and references only. Evaluation and
the golden fixtures stay in I2. The parser caps recursion depth
(`MAX_PARSE_DEPTH`): a deliberate Rust-only safety limit, because Swift's
parser overflows its stack on pathological nesting and no formula the Swift
oracle accepts is rejected.

**Tests:** port `ConfigurationDecodingTests` and
`AttractorCatalogValidationTests`; assert all 12 catalog entries decode;
malformed JSON yields typed errors.

**Verify:** `cargo test -p attractor-core`.

### I2 — Formula parser (M)

**Goal:** evaluate the catalog formulas identically.

**Deliverables:** evaluation over the AST built in I1, the function table
matching `Expression.supportedFunctions` (semantics: domain errors, `ln`/`log`
as the natural logarithm), typed errors.

**Tests:** port `ExpressionTests`; golden fixtures for every catalog formula
at 8 sample states, tolerance 1e-12; error-kind tests for unknown
functions/variables and malformed input.

**Verify:** `cargo test -p attractor-core`; fixture generator command
documented and reproducible.

### I3 — Numerical core (M)

**Goal:** the integration, randomness and buffering contract is identical.

**Deliverables:** `ODESolver` (Euler + RK4), `SplitMix64` (bit-exact),
`Randomizer`, `AttractorSelection` (the data aggregate the randomizer returns;
model building stays in I4), `SimulationStepScheduler`, `TraceBuffer` (ring),
`RestartLimiter`. `RandomSource` reproduces the Swift standard library's range
drawing (Lemire's `next(upperBound:)`, `Int.random(in:)`, the 53-bit
`Double.random(in:using:)`), so a seed selects the same values bit for bit.

**Tests:** port `ODESolverTests`, `SplitMix64Tests` (exact sequences),
`TraceBufferTests` + property tests, `RestartLimiterTests`,
`SimulationStepSchedulerTests`; short-horizon golden trajectories for Lorenz,
Chua and Aizawa (1e-9 relative); a seeded-selection fixture from the Swift
oracle compared exactly; long-run invariant tests (finite, bounded).

**Verify:** `cargo test -p attractor-core`.

### I4 — Engine, selection and health monitor (S/M)

**Goal:** a seeded end-to-end simulation matches Swift bit-for-bit on the RNG
and within tolerance on the trace.

**Deliverables:** `AttractorEngine`, `AttractorModel` (formulas bound to
parameters, plus `AttractorSelection::build_model`), `SimulationHealthMonitor`,
`DebugConfig`.

**Tests:** port `AttractorEngineTests`, `SimulationHealthMonitorTests`
(fixed point, limit cycle, divergence, non-finite states), determinism tests
(same seed ⇒ same fixture as the Swift engine).

**Verify:** `cargo test -p attractor-core`.

### I5 — View layer, no GPU (S/M)

**Goal:** camera and ribbon geometry with the current visual semantics.

**Deliverables:** orbit camera ported from `CameraController` with the same
constants (random start azimuth, elevation `π/5`, framing factor `2.1`,
smoothing `0.06`, framing refresh every 6 frames); trail builder with the
16 384-vertex budget, uniform subsampling, head preservation, world-space
ribbon radius derived from the trail extent, and all four color modes
(`trail`, `age`, `depth`, `spatial`); overlay content model.

**Tests:** camera matrix orthonormality and golden positions; vertex counts
and stride; head always present; color-ramp golden values for all four modes;
empty/short trace handling.

**Verify:** `cargo test -p attractor-view`.

### I6 — wgpu renderer and CLI capture (M/L)

**Goal:** the visual renders headlessly and can write a PNG.

**Deliverables:** `attractor-gpu` pipeline (per-vertex colors, world-space
ribbon radius derived from the trail extent, as in `TraceRenderer`); text
overlay (glyphon) on native; `attractor-cli` reaching flag parity with the
Swift demo, i.e. covering at least `--attractor`, `--seed`, `--screenshot`,
`--record`, `--record-seconds`, `--frames`, `--frame-stride`, `--width`,
`--height`, `--capture-scale`, `--param/-p`, `--initial-state`,
`--cam-azimuth`, `--cam-elevation`, `--cam-speed`, `--trail-color-speed`,
`--trail-length`, `--warmup-steps`, `--step`, `--speed`, `--color-mode`, and
the non-zero exit on a degenerate run that `docs/capture_all.sh` relies on.

**Tests:** headless offscreen render on macOS CI writes a non-blank PNG;
geometry-buffer snapshot test (primary, deterministic); loose perceptual diff
against a committed reference image (secondary, GPU-variance tolerant).

**Verify:** `cargo run -p attractor-cli -- --attractor lorenz --seed 42 --screenshot out.png --frames 120`; CI artifact inspected manually once.

### I7a — Player and C ABI (M)

**Goal:** one reusable runtime behind a stable C interface.

**Deliverables:** `attractor-player` (`tick`, `render`, `resize`, `restart`)
and `attractor-ffi` (C ABI + generated header). The player is
**one instance per surface**: every display gets its own player, its own
random selection and its own camera, matching the current macOS host (one
`ScreensaverView` per screen). Windows `/s` follows the same convention.

**Tests:** offscreen player frame equals the CLI frame for the same seed and
frame count; FFI smoke test through the C ABI.

**Verify:** `cargo test -p attractor-player -p attractor-ffi`.

### I7b — macOS host migration (M)

**Goal:** the `.saver` runs on the Rust player through a `CAMetalLayer`.

**Deliverables:** Swift view hosts the Metal layer and drives the player;
SceneKit renderer removed (old path available via `git revert` if needed);
overlay rendered by the engine; the `.appex`/container-app target and its
sources removed, leaving only the classic `.saver` (D10). The pre-migration
commit is pinned in `rust/STATUS.md` as the visual reference.

**Tests/verification:** `cd Core && swift test` (oracle still green);
`cd Demo && swift build`; `xcodegen generate` +
`xcodebuild -scheme AttractorSaver`; manual checklist: preview pane,
fullscreen, multi-display, wake/sleep, health-monitor restart.

**Done when:** manual visual comparison against the pinned pre-migration
build shows no regression; documented in `rust/STATUS.md`.

### I8 — Windows `.scr` (M)

**Deliverables:** `attractor-windows-saver` with `/s`, `/c`, `/p HWND`,
multi-monitor support, any-input exit, settings persisted under
`%APPDATA%`, and an exe renamed to `.scr`.

**Tests:** shared crate tests on `windows-latest`; `--self-test` headless
render (WARP); argument-parsing tests. The manual install checklist (Windows
11) is written here and executed with the friend at the milestone; until then
the iteration is "done (manual: pending)".

**Verify:** CI windows job builds and runs the self-test.

### I9 — Web demo on GitHub Pages (M)

**Deliverables:** `attractor-wasm` + `web/` static page: canvas, attractor
picker/search, seed, URL parameters; Pages deploy workflow.

**Tests:** wasm build in CI; Playwright test loads the page, renders N frames,
asserts the canvas is non-blank and the console is clean.

**Verify:** CI green; URL live on Pages; manual check in Chrome/Safari/Firefox.

### I10 — Linux (M)

**Deliverables:** xscreensaver hack binary (`-root`, `-window-id`),
fullscreen runner, `.desktop` file, install notes for xscreensaver and
`swayidle`/`hypridle`.

**Tests:** ubuntu CI build; headless screenshot under llvmpipe; hack-argument
tests. Manual checks on Debian/Ubuntu (X11 and Wayland) are scheduled with the
friend at the milestone.

### I11 — Android live wallpaper (M/L)

**Deliverables:** Kotlin `WallpaperService` + EGL surface, JNI bridge to the
player, attractor picker activity, preferences, `.apk`.

**Tests:** Rust bridge tests; `gradle assembleDebug` + lint in CI; manual
check on the emulator and, at the milestone, on the friend's phone.

### I12 — Distribution, docs and cleanup (M)

**Deliverables:** README funnel (animated preview, demo link, OS install
table), GitHub topics, release workflow producing macOS/Windows/Linux/Android
artifacts with checksums; catalog moved to `data/attractors.json`; README
images re-captured from `attractor-cli` through `docs/capture_all.sh` and
committed as the new visual baseline; `Core/`, `Render/` and `Demo/` retired
after the oracle is archived as a git tag/branch, leaving `Host/` as the only
Swift code; `AGENTS.md` rewritten for the Rust layout; the notarization
decision (D11) re-opened before the first macOS release.

**Verify:** release dry run; checksums; install checklist per OS on a clean
VM; `cargo test --workspace` and the remaining Swift host build both green.

## 8. Protocol for a fresh session

1. `git checkout rust-port` and read `rust/STATUS.md`.
2. Read the unchecked iteration in this document (or its successor).
3. Re-run the previous iteration's verification command to confirm the
   baseline before changing anything.
4. Implement only that iteration; add tests; run its verification command.
5. Update `rust/STATUS.md` (checkbox + notes/risks) and commit atomically.
6. If a decision in this document turns out wrong, amend the document and
   note it in the commit — do not silently diverge.

## 9. Left to the builder

- The exact ribbon-radius formula and anti-aliasing approach; the
  world-space semantics are fixed, the constants are matched to
  `TraceRenderer.ribbonRadius` at implementation time.
- Text renderer crate choice on native (`glyphon` assumed) and its font.
- Settings storage format per platform.
- Windows signing: whether to buy a certificate in I12 or ship unsigned with
  a SmartScreen note.
- Linux packaging breadth (tarball first vs AUR/deb immediately).
- Android minSdk and whether to target F-Droid in I12.
