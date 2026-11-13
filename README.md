# Strange Attractor Screensaver

A mesmerizing screen saver that simulates and renders **strange attractors** —
chaotic dynamical systems that trace infinitely intricate, never-repeating
paths in 3D space. One Rust engine, five places to watch it: macOS, Windows,
Linux, Android and your browser.

[![CI](https://github.com/xekchansky/strange-attractor-screensaver/actions/workflows/ci.yml/badge.svg)](https://github.com/xekchansky/strange-attractor-screensaver/actions/workflows/ci.yml)
[![Live demo](https://img.shields.io/badge/live%20demo-browser-brightgreen)](https://xekchansky.github.io/strange-attractor-screensaver/)

![Aizawa attractor animation](docs/images/strange-attractor-aizawa-screensaver-animation.gif)

Every launch picks a **random attractor** from a catalog of 12 classic chaotic
systems, randomizes its parameters and camera, and draws the trajectory as a
glowing gradient trail. The live formulas and parameter values are displayed in
the corner — a screen saver that is also a small piece of mathematical art.

## Try it now

No install, no build: **[open the live demo](https://xekchansky.github.io/strange-attractor-screensaver/)**
in a browser with WebGPU (or WebGL2). Pick an attractor, copy the URL to share
the exact seed.

## Install

Every download is ad-hoc signed and **not notarized**. See the platform note
before the first launch.

| Platform | Download | Before the first launch |
| --- | --- | --- |
| **macOS** 13+ | [`AttractorSaver.saver.zip`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Clear the quarantine flag: `xattr -r -d com.apple.quarantine ~/Downloads/AttractorSaver.saver` (see [macOS](#macos)). |
| **Windows** 10+ | [`StrangeAttractor.scr.zip`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Unzip, right-click `StrangeAttractor.scr` → **Install**. SmartScreen may ask; choose **More info → Run anyway** (see [Windows](#windows)). |
| **Linux** (x86_64) | [`strange-attractor-linux-x86_64.tar.gz`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Unpack and read the bundled `README.md`; the archive holds the saver, the capture CLI and the `.desktop` entry (see [Linux](#linux)). |
| **Android** 8+ | [`strange-attractor-android.apk`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Allow installing from the browser, then choose **Strange Attractor** in *Settings → Display → Live wallpaper* (see [Android](#android)). |
| **Browser** | [Live demo](https://xekchansky.github.io/strange-attractor-screensaver/) | Nothing to install. |

All archives are covered by one `SHA256SUMS` file in the same release:

```bash
sha256sum -c SHA256SUMS        # Linux
shasum -a 256 -c SHA256SUMS    # macOS
```

### macOS

1. Download the zip and the checksum file, unzip (Safari does this on its own)
   and optionally verify the download with `shasum -a 256 -c SHA256SUMS`.
2. Clear the quarantine flag so Gatekeeper lets the ad-hoc signed bundle run:
   `xattr -r -d com.apple.quarantine ~/Downloads/AttractorSaver.saver`.
   If a Homebrew `xattr` shadows the system one and fails with
   `bad interpreter`, call `/usr/bin/xattr` explicitly. Alternatively, try to
   open the bundle once and allow it via **System Settings → Privacy &
   Security → Open Anyway**.
3. Double-click `AttractorSaver.saver` — macOS opens it in **System Settings →
   Wallpaper → Screen Saver**. Alternatively, copy it to
   `~/Library/Screen Savers/`.
4. Select **AttractorSaver** in the screen saver list.

macOS 26 additionally requires the bundle to carry a signature (ad-hoc is
enough) — an unsigned `.saver` is killed by the host with
`Code Signature Invalid`. Notarization needs a paid Apple Developer account and
is deliberately postponed (design decision D11).

### Windows

1. Download and unzip `StrangeAttractor.scr.zip`.
2. Right-click `StrangeAttractor.scr` → **Install**, or copy it to
   `C:\Windows\System32` by hand and pick **Strange Attractor** in
   *Settings → Personalization → Lock screen → Screen saver settings*.
3. The binary is unsigned, so SmartScreen shows a warning for a fresh download;
   choose **More info → Run anyway**.

The `.scr` accepts the Windows screen saver protocol: `/s` runs it, `/p HWND`
draws into the control panel's preview, `/c` opens the small settings window
(the attractor choice is stored in
`%APPDATA%\StrangeAttractor\settings.json`), and an empty command line opens
the settings.

### Linux

The tarball contains `attractor-linux-saver` (both an xscreensaver hack and a
fullscreen runner), `attractor-cli` (capture and demo tool),
`strange-attractor.desktop` and the Linux `README.md` with the full recipe.
The short version:

```bash
install -Dm755 attractor-linux-saver ~/.local/bin/attractor-linux-saver
install -Dm644 strange-attractor.desktop ~/.local/share/applications/strange-attractor.desktop
attractor-linux-saver --fullscreen      # Wayland or any launcher
```

For xscreensaver, add `"Strange Attractor" attractor-linux-saver -root` to
`~/.xscreensaver` and restart the daemon. The settings file follows XDG:
`${XDG_CONFIG_HOME:-~/.config}/strange-attractor/settings.json`. The tarball is
built against the CI runner's glibc; on an older distribution build from source
instead.

### Android

1. Download `strange-attractor-android.apk` and open it; allow installing from
   this source when Android asks.
2. Open **Settings → Display → Live wallpaper → Strange Attractor**. The picker
   inside the wallpaper preferences chooses the attractor (or *Random each
   time*).

The published APK is signed with the **debug key**, because release signing
needs a keystore that cannot live in the repository. It installs and runs, but
play-store distribution is not set up. The wallpaper renders through GLES, one
player per visible surface.

## Features

- **12 strange attractors** — Lorenz, Aizawa, Rössler, Thomas, Chen, Lü, Chua,
  Halvorsen, Moore–Spiegel and the Sprott family (see
  [the catalog](#the-attractor-catalog)).
- **One engine, five shells** — the same Rust core and `wgpu` renderer draw the
  macOS `.saver`, the Windows `.scr`, the Linux hack, the Android wallpaper and
  the browser demo.
- **Data-driven design** — attractors are described in a single JSON catalog
  (formulas, parameter ranges, initial states), not in code. Adding a new one
  requires no code changes.
- **Runge–Kutta 4 integration** of the ODE system, with the number of steps per
  frame bounded so a sleep/wake stall cannot freeze the animation.
- **Self-healing** — a watchdog detects degenerate runs (divergence, collapse,
  stable periodic orbits, non-finite states) and silently restarts with a fresh
  random selection.
- **4 trail color modes** — rainbow along the trail, monochrome aging, depth
  from the camera, or spatial coloring.
- **Formula parsing at runtime** — the `dx/dy/dz` equations are parsed from
  strings, so the catalog stays pure data.
- **Seeded captures** — `attractor-cli` reproduces any frame exactly
  (`--attractor`, `--seed`, `--screenshot`), which is how the images below and
  the regression fixtures are made.

## The Attractor Catalog

| Attractor | Equations | Parameters | Preview |
|---|---|---|---|
| **Lorenz** | `dx = a(y − x)`<br>`dy = x(r − z) − y`<br>`dz = xy − bz` | a, b, r | ![Lorenz](docs/images/attractor-lorenz-simulation.png) |
| **Aizawa** | `dx = (z − b)x − dy`<br>`dy = dx + (z − b)y`<br>`dz = c + az − z³/3 − (x² + y²)(1 + ez) + fz x³` | a, b, c, d, e, f | ![Aizawa](docs/images/attractor-aizawa-simulation.png) |
| **Rössler** | `dx = −y − z`<br>`dy = x + ay`<br>`dz = b + z(x − c)` | a, b, c | ![Rössler](docs/images/attractor-rossler-simulation.png) |
| **Thomas** | `dx = sin(y) − bx`<br>`dy = sin(z) − by`<br>`dz = sin(x) − bz` | b | ![Thomas](docs/images/attractor-thomas-simulation.png) |
| **Chen** | `dx = a(y − x)`<br>`dy = (c − a)x − xz + cy`<br>`dz = xy − bz` | a, b, c | ![Chen](docs/images/attractor-chen-simulation.png) |
| **Lü** | `dx = a(y − x)`<br>`dy = −xz + cy`<br>`dz = xy − bz` | a, b, c | ![Lü](docs/images/attractor-lu-simulation.png) |
| **Chua** (double scroll) | `dx = α(y − x − f(x))`<br>`dy = x − y + z`<br>`dz = −βy` with piecewise-linear `f(x)` | α, β, m₀, m₁ | ![Chua](docs/images/attractor-chua-simulation.png) |
| **Halvorsen** (cyclically symmetric) | `dx = −ax − 4y − 4z − y²`<br>`dy = −ay − 4z − 4x − z²`<br>`dz = −az − 4x − 4y − x²` | a | ![Halvorsen](docs/images/attractor-halvorsen-simulation.png) |
| **Moore–Spiegel** (2nd-order oscillator) | `dx = 10y`<br>`dy = 35z`<br>`dz = −z − (T − R + Rx²)y/35 − (T/350)x` | R, T | ![Moore–Spiegel](docs/images/attractor-moore_spiegel-simulation.png) |
| **Sprott A** | `dx = y`<br>`dy = −x + yz`<br>`dz = 1 − y²` | fixed | ![Sprott A](docs/images/attractor-sprott_a-simulation.png) |
| **Sprott B** | `dx = yz`<br>`dy = x − y`<br>`dz = 1 − xy` | fixed | ![Sprott B](docs/images/attractor-sprott_b-simulation.png) |
| **Sprott C** | `dx = yz`<br>`dy = x − y`<br>`dz = 1 − x²` | fixed | ![Sprott C](docs/images/attractor-sprott_c-simulation.png) |

Each attractor's parameters and simulation hyperparameters (step size, trail
length, camera speed, color cycle speed) are randomized within curated ranges
in [`data/attractors.json`](data/attractors.json). The catalog is validated when
it loads — formulas must parse, ranges must be sane and only implemented
functions are allowed — and unit tests integrate every bundled entry. Chaotic
families with tiny basins of attraction (e.g. Sprott D–J, Rabinovich–Fabrikant)
are excluded because they diverge from random starts. The images above are
regenerated with
[`docs/capture_all.sh`](docs/capture_all.sh) from the Rust capture CLI, with a
fixed seed and pinned views.

## How It Works

```text
data/attractors.json ──▶ attractor-core ──▶ attractor-view ──▶ attractor-gpu
   catalog + formulas     RK4 engine,        camera, trail       wgpu ribbon +
                          health monitor    geometry, overlay   overlay text
                                                  │
                                                  ▼
                                            attractor-player
                                     tick / render / resize / restart
                                                  │
        ┌──────────────┬──────────────┬────────────┴──────────┬──────────────┐
        ▼              ▼              ▼                       ▼              ▼
   macOS .saver   Windows .scr   Linux hack / runner    Android JNI     wasm demo
     (FFI +         (Win32)          (X11/winit)          (Wallpaper       (browser
    AppKit view)                                          Service)        canvas)
```

- **`rust/crates/attractor-core`** — pure, UI-free engine: data model, formula
  parser (recursive descent), RK4 solver, engine with a ring-buffer trail,
  randomizer and health monitor.
- **`rust/crates/attractor-view`** — GPU-free view layer: orbiting camera with
  auto-framing, gradient ribbon geometry, all four color modes, overlay model.
- **`rust/crates/attractor-gpu`** — `wgpu` pipelines: multisampled ribbon
  rendering, the text overlay, PNG readback and the platform surface targets.
- **`rust/crates/attractor-player`** — the runtime every shell embeds.
- **`rust/crates/attractor-ffi`** — the C ABI used by the macOS host and the
  Android JNI bridge; **`attractor-wasm`** — the browser glue.
- **`rust/crates/attractor-windows-saver`** and
  **`rust/crates/attractor-linux-saver`** — the native shells.
- **`rust/shells/android`** — the Kotlin `WallpaperService` and picker.
- **`Host/`** — the macOS `.saver`, a thin Swift/AppKit view over the C ABI.

The player re-rolls the attractor when the watchdog detects a bad run (e.g. the
trajectory collapses to a point or settles into a periodic loop), so it never
sits frozen on a broken frame.

## Build from source

Rust (stable, edition 2024) is all the engine needs; the shells add Xcode, the
Windows SDK or the Android SDK/NDK.

```bash
git clone https://github.com/xekchansky/strange-attractor-screensaver.git
cd strange-attractor-screensaver

# Engine tests and the workspace check
cd rust
cargo test --workspace

# Capture a frame (needs a real GPU, so run it in a GUI session)
cargo run --release -p attractor-cli -- \
  --attractor lorenz --seed 42 --screenshot lorenz.png
```

The capture CLI mirrors the old demo app's flags: `--attractor`, `-p name=value`
(repeatable), `--step`, `--speed`, `--cam-speed`, `--trail-color-speed`,
`--trail-length`, `--color-mode`, `--screenshot`, `--record`,
`--record-seconds`, `--frame-stride`, `--capture-scale`, `--warmup-steps`,
`--cam-azimuth`, `--cam-elevation`, `--initial-state`, plus `--seed`,
`--frames`, `--width` and `--height`.

### macOS screen saver

```bash
cd Host && xcodegen generate && cd ..
./scripts/package_release.sh            # writes dist/AttractorSaver.saver.zip
```

`ARCHS="arm64 x86_64"` builds the universal bundle (needs both Rust standard
libraries: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`).

### Windows screen saver

```powershell
cargo build --release -p attractor-windows-saver
Copy-Item rust\target\release\attractor-windows-saver.exe StrangeAttractor.scr
```

### Linux

```bash
cd rust
cargo build --release -p attractor-linux-saver -p attractor-cli
./scripts/package_linux.sh              # from the repository root
```

See [`rust/shells/linux/README.md`](rust/shells/linux/README.md) for
xscreensaver, `swayidle`/`hypridle` and settings details.

### Android

```bash
cd rust/shells/android
rustup target add aarch64-linux-android x86_64-linux-android
cargo install --locked cargo-ndk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
./build-native.sh                       # jniLibs for the APK
./gradlew assembleDebug
```

See [`rust/shells/android/README.md`](rust/shells/android/README.md) for the
manual emulator/device checklist.

### Browser demo

```bash
bash web/build.sh
cd web && npm ci && npm test
```

## Requirements and tested setup

- **Engine:** stable Rust (edition 2024), no GPU needed for the tests — the
  pipeline tests use `wgpu`'s `noop` backend. Captures and shells need a real
  GPU adapter.
- **macOS:** 13 or newer, Apple Silicon or Intel; Xcode and
  [xcodegen](https://github.com/yonaskolb/XcodeGen) for the bundle. Verified on
  macOS 26 with the Rust stable toolchain and Xcode 26.
- **Windows:** 10 or newer with DirectX 12 (or Vulkan). CI builds and renders
  through WARP on `windows-latest`.
- **Linux:** X11 or Wayland with Vulkan or GLES; CI runs the X11 paths under
  Xvfb with Mesa's software Vulkan driver.
- **Android:** 8.0+ (API 26), `arm64-v8a` and `x86_64`; NDK 27.2.
- **Browser:** WebGPU, with an automatic WebGL2 fallback.

## Troubleshooting

- **The macOS screen saver does not appear in System Settings.** Re-copy the
  bundle with `ditto` (not `cp -R`) and restart the wallpaper agent:
  `killall WallpaperAgent`. The system caches the screen saver list.
- **The macOS screen saver shows "Invalid Page" or a black preview.** A stale
  copy is usually still loaded by the host. Quit System Settings, remove the
  bundle, copy it again and reopen the settings pane.
- **The macOS download is blocked by Gatekeeper.** See the quarantine step in
  [macOS](#macos).
- **Windows shows "Windows protected your PC".** The `.scr` is unsigned; use
  **More info → Run anyway**, or buy a code-signing certificate and sign the
  binary yourself.
- **A shell starts and immediately exits.** It needs a working GPU adapter;
  run `attractor-cli --screenshot` first — the error names the exact reason
  (`no GPU adapter`, a surface error, a catalog error).

## Notes & Known Issues

- Installing an **unsigned** `.saver` on macOS 26 can wedge the session's
  global hotkeys (Cmd+Tab, input switching) until reboot — see
  [the write-up](.serena/memories/macos_input_lockup.md). Release bundles are
  ad-hoc signed, which the system accepts.
- The published Android APK carries the debug signature (see
  [Android](#android)); macOS bundles are not notarized (D11); the Windows
  `.scr` is unsigned.

## Documentation

- [README in Russian](README.ru.md)
- Contributor and coding-agent notes: [`AGENTS.md`](AGENTS.md)
- Iteration status, decisions and deferred work:
  [`rust/STATUS.md`](rust/STATUS.md)
- Cross-platform design and ADRs:
  [`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md)
- Capture script for documentation assets: [`docs/capture_all.sh`](docs/capture_all.sh)
- Release packaging: [`scripts/package_release.sh`](scripts/package_release.sh),
  [`scripts/package_linux.sh`](scripts/package_linux.sh)
- Security policy: [`SECURITY.md`](SECURITY.md)

## License

[MIT](LICENSE) © Khakayu Alexandr
