# attractor-windows-saver

The Windows `.scr` shell: one borderless window and one player per monitor,
driven by the same Rust player the macOS `.saver` uses. Windows starts the
executable with `/s`, `/c` or `/p`; the command line parser, the settings file
and the offscreen self-test are platform-neutral and tested on every OS.

## Build and package

```powershell
cd rust
cargo build --release -p attractor-windows-saver
copy target\release\attractor-windows-saver.exe StrangeAttractor.scr
```

The `.scr` extension is what makes the file show up in the Screen Saver
Settings page. The binary is unsigned, so the first download triggers a
SmartScreen warning; a signing certificate is a release-time decision (design
D11).

## Install

1. Right-click `StrangeAttractor.scr` → **Install**, or copy it to
   `C:\Windows\System32\`.
2. Open **Settings → Personalization → Lock screen → Screen saver settings**
   and pick **Strange Attractor**.
3. **Settings…** opens the attractor picker. **Preview** starts the saver in
   the small panel; waiting for the idle timeout starts it on every monitor.

## Modes

| Invocation | Behaviour |
| --- | --- |
| `/s` | draw on every monitor until a key, a mouse button, the wheel or pointer motion arrives |
| `/c`, `/c:HWND` | open the settings window, optionally owned by `HWND` |
| `/p HWND` | draw inside the control panel's preview control |
| no arguments | same as `/c` (a double-click on the file) |
| `--self-test` | render a frame offscreen and report the adapter and lit pixels |
| `--self-test --noop` | validate the pipelines through wgpu's noop backend |
| `--help` | print the usage text |

The first pointer motion within 250 ms of the start is ignored; Windows can
deliver a leftover `WM_MOUSEMOVE` from the click that started the saver.

## Settings

The settings file is plain JSON, so it can be edited without the picker:

```text
%APPDATA%\StrangeAttractor\settings.json
```

```json
{
  "attractor": "lorenz"
}
```

An absent file, an empty value or an id that is not in the catalog means
"random attractor on every launch". Unknown keys are ignored, so the format can
grow.

## Self-test

```powershell
StrangeAttractor.scr --self-test
```

The run is seeded, warms up for 5 000 integration steps, renders 60 frames at
640×480 and prints the adapter, the backend and the number of lit pixels. It
exits 0 and prints `skipped` when the machine has no GPU adapter, which is what
the CI job tolerates on an unusual runner; `--self-test --noop` exercises the
pipelines without a GPU and always runs.

The binary is a windowed (GUI-subsystem) program on purpose: Windows would
otherwise create a console window for the whole saver session. The report still
reaches a pipe, so `cargo run -p attractor-windows-saver -- --self-test` and CI
capture it; from a terminal, pipe the command (`… --self-test | Out-String`).

## Manual checklist (Windows 11, to be executed with the reviewer)

- [ ] `StrangeAttractor.scr --self-test` prints a D3D12/WARP adapter and a
      non-zero lit-pixel count.
- [ ] The file appears in Screen Saver Settings after right-click → Install.
- [ ] **Preview** animates in the settings panel and stops when the panel
      closes.
- [ ] **Settings…** lists every catalog attractor; choosing one and pressing OK
      writes `%APPDATA%\StrangeAttractor\settings.json`, and the next run starts
      that attractor.
- [ ] Editing the file to an unknown id falls back to random attractors.
- [ ] Idle timeout covers every monitor with its own attractor and camera.
- [ ] Any key press, mouse button, wheel notch or pointer motion ends the
      session within one frame; the cursor is visible again.
- [ ] Unplugging a second monitor while the saver runs ends the session within
      one frame instead of hanging or presenting into the removed display.
