# attractor-linux-saver

The Linux shell: an xscreensaver hack and a fullscreen runner over the same
Rust player the macOS `.saver` and the Windows `.scr` use. The command-line
parser, the settings file and the offscreen self-test are platform-neutral and
tested on every OS; the window backends build only on Linux.

The install story — xscreensaver, `swayidle`, `hypridle`, the `.desktop` file
and the manual checklist — lives in
[`shells/linux/README.md`](../../shells/linux/README.md).

## Build

```bash
cd rust
cargo build --release -p attractor-linux-saver
```

## Modes

| Invocation | Behaviour |
| --- | --- |
| `-root` | xscreensaver root mode: draw on the screen until any input |
| `-window-id N` | draw inside the window xscreensaver mapped; leave when it is destroyed |
| `--fullscreen` | fullscreen runner through winit (X11 and Wayland) |
| no arguments | same as `--fullscreen` |
| `--frames N` | stop after `N` frames (CI smoke test) |
| `--self-test` | render a frame offscreen and report the adapter and lit pixels |
| `--self-test --noop` | validate the pipelines through wgpu's noop backend |
| `--help` | print the usage text |

xscreensaver's own flags (`-delay`, `-fps`, `-mono`, `-visual`, `-install`,
`-noinstall`, `-verbose`) are accepted and ignored.

`-root` does not draw directly into the X root window: Vulkan presentation
into the root is unreliable, so the shell creates an override-redirect window
that covers the screen. The visible result is the same, and the window goes
away with the process.

## Settings

```text
$XDG_CONFIG_HOME/strange-attractor/settings.json   # usually ~/.config/...
```

```json
{
  "attractor": "lorenz"
}
```

An absent file, an empty value or an unknown id means "random attractor on
every launch"; unknown keys are ignored.

## Self-test

```bash
cargo run -p attractor-linux-saver -- --self-test
```

The run is seeded, warms up for 5 000 integration steps, renders 60 frames at
640×480 and prints the adapter, the backend and the number of lit pixels. It
exits 0 and prints `skipped` when the machine has no GPU adapter;
`--self-test --noop` exercises the pipelines without a GPU and always runs.

## Tests

```bash
cd rust
cargo test -p attractor-linux-saver
```

The arguments, the settings file, the input policy and the display seeds are
covered on every OS. The X11 and winit backends are exercised on the ubuntu CI
job under Xvfb with Mesa's software Vulkan driver.
