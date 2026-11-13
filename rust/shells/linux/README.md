# Strange Attractor on Linux

One binary covers both Linux stories:

- **X11 / xscreensaver** — a regular xscreensaver hack. xscreensaver starts it
  with `-root` or `-window-id N` and kills it when the user comes back.
- **Wayland and desktop launchers** — a fullscreen runner. Wayland has no
  screen saver API, so an idle hook (`swayidle`, `hypridle`) or the
  `.desktop` file starts `attractor-linux-saver --fullscreen`.

## Build and install

```bash
cd rust
cargo build --release -p attractor-linux-saver
install -Dm755 target/release/attractor-linux-saver ~/.local/bin/attractor-linux-saver
install -Dm644 shells/linux/strange-attractor.desktop ~/.local/share/applications/strange-attractor.desktop
```

`~/.local/bin` has to be on `PATH` for the `.desktop` file and the idle hooks.
A system-wide install uses `/usr/local/bin` and
`/usr/local/share/applications`.

## xscreensaver (X11)

Add the hack to `~/.xscreensaver` (or use **xscreensaver-demo → Advanced →
Programs**):

```text
programs: \
  "Strange Attractor" attractor-linux-saver -root \
  maze -root \
  ...
```

Then restart the daemon:

```bash
xscreensaver-command -restart
```

`-root` makes the saver cover the screen with its own override-redirect
window; xscreensaver's `-window-id` mode (used by the preview thumbnail) is
handled by creating the swap chain on the window xscreensaver mapped and
leaving when that window is destroyed or unmapped. xscreensaver's own flags
(`-delay`, `-fps`, `-mono`, `-visual`, `-install`, `-noinstall`, `-verbose`)
are accepted and ignored.

## swayidle (sway, Wayland or X11)

```text
exec swayidle -w \
  timeout 300 'attractor-linux-saver --fullscreen' \
  resume 'pkill -f attractor-linux-saver'
```

## hypridle (Hyprland, Wayland)

```text
listener {
    timeout = 300
    on-timeout = attractor-linux-saver --fullscreen
    on-resume = pkill -f attractor-linux-saver
}
```

## Desktop launcher

`strange-attractor.desktop` starts the fullscreen runner on demand. The runner
picks its window backend at runtime — X11 on an X11 session, native Wayland on
a Wayland session — so a hotkey or a menu entry works the same way
everywhere.

## Settings

The settings file is plain JSON and may be edited by hand:

```text
$XDG_CONFIG_HOME/strange-attractor/settings.json   # usually ~/.config/...
```

```json
{
  "attractor": "lorenz"
}
```

An absent file, an empty value or an id that is not in the catalog means
"random attractor on every launch". Unknown keys are ignored. The catalog ids
are the ones the web demo and the other shells list (`lorenz`, `aizawa`,
`chua`, `thomas`, …).

## Behaviour

| Invocation | Behaviour |
| --- | --- |
| `-root` | draw on the screen until any key, mouse button, wheel notch or pointer motion |
| `-window-id N` | draw inside window `N`; leave when the window is destroyed or unmapped |
| `--fullscreen` | fullscreen runner on every monitor; same input exit |
| no arguments | same as `--fullscreen` |
| `--frames N` | stop after `N` frames (used by CI and for a quick smoke test) |
| `--self-test` | render a frame offscreen and report the adapter and lit pixels |
| `--self-test --noop` | validate the pipelines through wgpu's noop backend |
| `--help` | print the usage text |

Pointer motion is ignored for 250 ms after the first frame, so the click that
started the keyboard shortcut or the launcher does not dismiss the saver.

## Manual checklist (Debian/Ubuntu, X11 and Wayland)

- [ ] `attractor-linux-saver --self-test` prints a Vulkan or GL adapter and a
      non-zero lit-pixel count; `--self-test --noop` validates the pipelines.
- [ ] xscreensaver lists **Strange Attractor**, the preview thumbnail animates,
      and the idle timeout covers the screen.
- [ ] The xscreensaver preview and the idle run leave on a key press, a mouse
      button, the wheel or pointer motion.
- [ ] `attractor-linux-saver --fullscreen` covers all monitors under X11; each
      monitor runs its own attractor and camera.
- [ ] On a Wayland session (sway or GNOME) the same command covers the screen
      and exits on input.
- [ ] The `.desktop` entry starts the saver from the application menu.
- [ ] Pinning `"attractor": "aizawa"` in the settings file runs Aizawa; a
      stale id falls back to random attractors.
