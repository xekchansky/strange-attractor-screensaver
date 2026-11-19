# README media recapture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recapture the README stills and the hero GIF without the formula overlay, in the Swift-era framing, into a local preview folder that is reviewed before anything lands in `docs/images`.

**Architecture:** A `show_overlay` flag on `PlayerOptions` (default `true`) lets any caller drop the corner text; the capture CLI exposes it as `--no-overlay`, and `docs/capture_all.sh` passes it for every still and every recorded frame. The GPU capture itself runs in the human partner's GUI session; the agent assembles the GIF, builds a Swift-vs-Rust comparison sheet and checks each frame for ink on the border.

**Tech Stack:** Rust (edition 2024), wgpu/glyphon via `attractor-gpu`, zsh, ffmpeg/sips for the GIF and the border check.

**Spec:** `docs/superpowers/specs/2026-11-20-readme-media-recapture-design.md`

## Global Constraints

- Keep logic in `attractor-core`; `PlayerOptions` lives in `attractor-player` and the CLI flag in `attractor-cli`.
- `attractor-view` stays GPU-free and deterministic. The camera FOV fix on `fix/camera-framing` is **not** part of this work; it is stashed for the capture and restored afterwards, uncommitted.
- Every public symbol gets a unit test; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` must be green in `rust/`.
- Nothing in `docs/images`, `README.md` or `README.ru.md` changes until the human partner approves a preview.
- Comments and identifiers in English. One atomic commit at the end, no push.
- Capture needs a real GPU and a GUI session; it cannot run in the agent's sandbox.

## Review Focus

- A plain capture without `--no-overlay` still burns the overlay into the frame (the flag must be opt-in).
- The live native shells (macOS, Windows, Linux, Android) keep their in-frame overlay (the default must stay `true`); the browser keeps its DOM overlay.
- `--record` honours the flag exactly like `--screenshot` (the hero frames are recorded, not snapshotted).
- `--no-overlay` parses in any argument position and does not disturb existing flag validation.
- A capture run writes only into the preview directory, never over `docs/images`.

---

### Task 1: `show_overlay` on `PlayerOptions`

**Files:**
- Modify: `rust/crates/attractor-player/src/player.rs`
- Modify: `rust/crates/attractor-wasm/src/web.rs:63`
- Test: `rust/crates/attractor-player/tests/player.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `PlayerOptions { pub show_overlay: bool }`, default `true`. `Player::render` and `Player::render_to_surface` pass `None` instead of `Some(&overlay)` when the flag is false.

- [ ] **Step 1: Write the failing unit test**

In the `mod tests` at the bottom of `rust/crates/attractor-player/src/player.rs`, extend `default_options_match_the_capture_surface`:

```rust
assert!(options.show_overlay, "the screensaver shows the overlay by default");
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd rust && cargo test -p attractor-player --lib default_options_match_the_capture_surface`
Expected: FAIL — no field `show_overlay`.

- [ ] **Step 3: Add the field and thread it through the render paths**

In `rust/crates/attractor-player/src/player.rs`:

```rust
/// Draw the formula overlay into captured and presented frames.
pub show_overlay: bool,
```

after `pub overlay_scale: f32`, and `show_overlay: true` in `impl Default`. In both `render` and `render_to_surface`, replace the overlay argument:

```rust
let overlay = OverlayModel::new(&self.selection, self.engine.state()).text();
let overlay = self.options.show_overlay.then_some(overlay.as_str());
...
renderer.render(context, self.builder.geometry(), &view_projection, overlay)?;
```

(`render_to_view` takes `Option<&str>` in both places.)

- [ ] **Step 4: Update the wasm literal**

`rust/crates/attractor-wasm/src/web.rs:63` builds `PlayerOptions` without `..Default`; add `show_overlay: false,` next to the `overlay_scale` comment (the browser draws its overlay as DOM text; the wasm build also compiles `attractor-gpu` without the `text` feature, so this stays correct if that ever changes). Leave every other constructor as it is: they all use `..PlayerOptions::default()`.

- [ ] **Step 5: Add the GPU-gated pixel test**

In `rust/crates/attractor-player/tests/player.rs`, next to `a_rendered_frame_is_not_blank`:

```rust
#[test]
fn hiding_the_overlay_inks_fewer_pixels_than_showing_it() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the overlay test: {error}");
            return;
        }
    };
    let mut shown = player(42, PlayerOptions { overlay_scale: 2.0, ..pinned_options() });
    let mut hidden = player(
        42,
        PlayerOptions { overlay_scale: 2.0, show_overlay: false, ..pinned_options() },
    );
    for player in [&mut shown, &mut hidden] {
        player.warm_up(5_000).expect("warm-up stays healthy");
        for _ in 0..60 {
            player.tick(FRAME_INTERVAL).expect("frame advances");
        }
    }
    let shown_pixels = lit_pixels(&shown.render(&context).expect("frame renders"));
    let hidden_pixels = lit_pixels(&hidden.render(&context).expect("frame renders"));
    assert!(shown_pixels > 0, "the ribbon must draw ink");
    assert!(
        hidden_pixels < shown_pixels,
        "the overlay must add ink: {hidden_pixels} hidden vs {shown_pixels} shown"
    );
}
```

- [ ] **Step 6: Run the tests**

Run: `cd rust && cargo test -p attractor-player`
Expected: PASS (the GPU test prints a skip when no adapter is available).

- [ ] **Step 7: Commit**

```bash
git add rust/crates/attractor-player/src/player.rs rust/crates/attractor-player/tests/player.rs rust/crates/attractor-wasm/src/web.rs
git commit -m "feat(player): let a caller hide the formula overlay"
```

---

### Task 2: `--no-overlay` in the capture CLI

**Files:**
- Modify: `rust/crates/attractor-cli/src/arguments.rs`
- Modify: `rust/crates/attractor-cli/src/run.rs`
- Test: `rust/crates/attractor-cli/tests/arguments.rs`

**Interfaces:**
- Consumes: `PlayerOptions::show_overlay` from Task 1.
- Produces: `CommandLineArguments { pub no_overlay: bool }`; `prepare` passes `show_overlay: !arguments.no_overlay`.

- [ ] **Step 1: Write the failing tests**

In `rust/crates/attractor-cli/tests/arguments.rs`:

```rust
#[test]
fn the_overlay_is_shown_unless_it_is_turned_off() {
    assert!(!parse(&[]).no_overlay);
    assert!(parse(&["--no-overlay", "--attractor", "lorenz", "--screenshot", "x.png"]).no_overlay);
}
```

Also add `assert!(!arguments.no_overlay);` to `an_empty_command_line_keeps_every_default` and assert that `CommandLineArguments::USAGE.contains("--no-overlay")` in `usage_lists_the_headless_capture_flags`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd rust && cargo test -p attractor-cli --test arguments`
Expected: FAIL — no field `no_overlay`, usage string missing the flag.

- [ ] **Step 3: Implement the flag**

In `rust/crates/attractor-cli/src/arguments.rs`:
- add `pub no_overlay: bool,` after `pub frames: Option<usize>,`, documented as "True when `--no-overlay` was passed: captures leave out the corner text.";
- add to `USAGE` after `--frames`: `      --no-overlay             hide the formula overlay in the capture`;
- in `parse`, start with `let mut no_overlay = false;`, add `"--no-overlay" => no_overlay = true,` (a value-less flag, like `--help`), and pass `no_overlay` in the returned struct.

In `rust/crates/attractor-cli/src/run.rs`, inside the `PlayerOptions { ... }` literal in `prepare`:

```rust
show_overlay: !arguments.no_overlay,
```

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p attractor-cli`
Expected: PASS. `tests/headless_snapshot.rs` and `tests/record_sequence.rs` still pass because they build options directly.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/attractor-cli/src/arguments.rs rust/crates/attractor-cli/src/run.rs rust/crates/attractor-cli/tests/arguments.rs
git commit -m "feat(cli): add --no-overlay for documentation captures"
```

---

### Task 3: Capture script and preview convention

**Files:**
- Modify: `docs/capture_all.sh`
- Modify: `.gitignore`
- Modify: `README.md`, `README.ru.md` (flag list only)

**Interfaces:**
- Consumes: `--no-overlay` from Task 2.
- Produces: `docs/capture_all.sh <binary> <output-dir>` writing overlay-free stills to `<output-dir>/screenshots` and overlay-free frames to `<output-dir>/aizawa-frames`.

- [ ] **Step 1: Pass the flag in the script**

In `docs/capture_all.sh`:
- the screenshot call becomes
  `capture_with_retry "$a" ${=$(capture_args "$a")} --no-overlay --screenshot "$OUT/screenshots/$a.png"`;
- the hero `--record` invocation gains `--no-overlay` next to `--capture-scale 1`;
- extend the header comment: "All captures pass `--no-overlay`: the README shows only the attractor. Preview runs write to an ignored `docs/previews/<label>` directory until the assets are approved."

- [ ] **Step 2: Ignore the preview directory**

Append to `.gitignore`:

```
# Local documentation previews, reviewed before they land in docs/images.
docs/previews/
```

- [ ] **Step 3: List the flag in both READMEs**

In `README.md` and `README.ru.md`, add `--no-overlay` to the capture-flag list, next to `--capture-scale`.

- [ ] **Step 4: Verify the script parses**

Run: `zsh -n docs/capture_all.sh`
Expected: no output, exit 0.

- [ ] **Step 5: Commit**

```bash
git add docs/capture_all.sh .gitignore README.md README.ru.md
git commit -m "docs(capture): record the README media without the overlay"
```

---

### Task 4: Capture the preview set (human partner runs the GPU part)

**Files:**
- Produces: `docs/previews/<label>/screenshots/*.png`, `docs/previews/<label>/aizawa-frames/*.png`

**Interfaces:**
- Consumes: the built CLI from Tasks 1–2 and the script from Task 3.

- [ ] **Step 1: Build the release CLI**

Run: `cd rust && cargo build --release -p attractor-cli`
Expected: `rust/target/release/attractor-cli` is fresh.

- [ ] **Step 2: Park the camera FOV fix for the duration of the capture**

Run: `git stash push -m "camera fov fix" -- rust/crates/attractor-view/src/camera.rs rust/crates/attractor-view/tests/camera.rs`
Expected: `git status --short` no longer lists those two files. Record the stash name.

- [ ] **Step 3: Rebuild after the stash**

Run: `cd rust && cargo build --release -p attractor-cli`
Expected: a rebuild picked up the Swift-era projection.

- [ ] **Step 4: Ask the human partner to run the capture**

Give them exactly:

```bash
cd /Users/akhakayu/Projects/personal/strange-attractor-screensaver
./docs/capture_all.sh rust/target/release/attractor-cli docs/previews/v1
```

Expected: twelve PNGs in `docs/previews/v1/screenshots`, 2400 PNGs in `docs/previews/v1/aizawa-frames`, exit 0. The agent cannot run this: the sandbox has no Metal adapter.

- [ ] **Step 5: Restore the camera fix**

Run: `git stash pop`
Expected: the two camera files are modified again; `cargo test -p attractor-view` passes.

- [ ] **Step 6: Check the preview set**

Run: `ls docs/previews/v1/screenshots | wc -l` and `ls docs/previews/v1/aizawa-frames | wc -l`
Expected: 12 and 2400.

---

### Task 5: Preview sheet, GIF and border check

**Files:**
- Create: `docs/previews/v1/swift/*.png` (originals from `swift-oracle`, ignored)
- Create: `docs/previews/v1/index.html` (ignored)
- Create: `docs/previews/v1/strange-attractor-aizawa-screensaver-animation.gif` (ignored)

**Interfaces:**
- Consumes: the capture set from Task 4.

- [ ] **Step 1: Extract the Swift-era originals**

```bash
mkdir -p docs/previews/v1/swift
for f in $(git ls-tree --name-only swift-oracle docs/images/ | grep 'attractor-.*\.png$'); do
  git show "swift-oracle:$f" > "docs/previews/v1/swift/$(basename "$f")"
done
```

Expected: twelve PNGs in `docs/previews/v1/swift`.

- [ ] **Step 2: Assemble the hero GIF**

```bash
mkdir -p docs/previews/v1/aizawa-640
for f in docs/previews/v1/aizawa-frames/frame_*.png; do
  sips -Z 640 "$f" --out "docs/previews/v1/aizawa-640/$(basename "$f")" >/dev/null
done
ffmpeg -y -framerate 50 -i docs/previews/v1/aizawa-640/frame_%05d.png \
  -vf "palettegen=max_colors=256:stats_mode=full" -update 1 docs/previews/v1/pal.png
ffmpeg -y -framerate 50 -i docs/previews/v1/aizawa-640/frame_%05d.png \
  -i docs/previews/v1/pal.png -lavfi "paletteuse=dither=bayer:bayer_scale=3" \
  docs/previews/v1/strange-attractor-aizawa-screensaver-animation.gif
```

Expected: a GIF under ~20 MB whose frame 600 shows no text in the lower-left corner.

- [ ] **Step 3: Measure border ink per still**

For each PNG compute the black-pixel percentage of a 3-pixel border strip on every side:

```bash
for f in docs/previews/v1/screenshots/*.png; do
  printf '%s' "$(basename "$f")"
  for crop in "iw:3:0:0" "iw:3:0:ih-3" "3:ih:0:0" "3:ih:iw-3:0"; do
    p=$(ffmpeg -v error -i "$f" -vf "crop=$crop,blackframe=amount=0:threshold=8" -f null - 2>&1 \
        | sed -n 's/.*pblack:\([0-9]*\).*/\1/p' | head -1)
    printf ' %s' "${p:-?}"
  done
  echo
done
```

Expected: `100` on all four edges for the eleven matching stills; Aizawa shows at least one edge below `100`, which is the objective clipping flag.

- [ ] **Step 4: Write the comparison sheet**

Create `docs/previews/v1/index.html`: a table with one row per attractor, columns "Swift (swift-oracle)" and "Rust (this run)" with the two images side by side at half width, plus a column with the four border-ink numbers. Add the GIF at the top with a link to the Swift tag's GIF for comparison. No external CSS or scripts.

- [ ] **Step 5: Hand the sheet to the human partner**

Ask them to open `docs/previews/v1/index.html` (they run `open docs/previews/v1/index.html`; the agent's sandbox blocks Launch Services). Ask which assets to keep and which to change.

- [ ] **Step 6: Retune the flagged attractors only**

For each rejected asset, change the recipe in `capture_args` (angle, trail length, start state) or the pinned parameters, then re-run only that one:

```bash
ATTRACTORS="aizawa" SKIP_HERO=1 ./docs/capture_all.sh rust/target/release/attractor-cli docs/previews/v2
```

Repeat Steps 3–5 for the new label. Do not touch `docs/images`.

---

### Task 6: Adopt the approved assets

**Files:**
- Modify: `docs/images/*.png`, `docs/images/strange-attractor-aizawa-screensaver-animation.gif`
- Modify: `.serena/memories/documentation_captures.md`, `rust/STATUS.md`

**Interfaces:**
- Consumes: the approved label from Task 5.

- [ ] **Step 1: Copy the approved files**

```bash
cp docs/previews/<label>/screenshots/*.png docs/images/attractor-<id>-simulation.png   # one by one, renamed
cp docs/previews/<label>/strange-attractor-aizawa-screensaver-animation.gif docs/images/
```

Expected: `git status` lists exactly the approved images.

- [ ] **Step 2: Confirm the overlay is gone and nothing clips**

Re-run the border-ink loop from Task 5 Step 3 against the copied `docs/images` files.
Expected: `100` on all four edges for every still.

- [ ] **Step 3: Update the notes**

- `.serena/memories/documentation_captures.md`: record that README captures pass `--no-overlay` (the Swift snapshot never included the AppKit label), that the Swift projection is kept for asset parity, and that the Swift-era Aizawa still was unpinned and is not reproducible.
- `rust/STATUS.md`: append the iteration entry with the ruling (Swift parity + overlay-free captures), the Aizawa decision, and the deferred camera FOV fix.

- [ ] **Step 4: Run the workspace gate**

Run: `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all green (GPU tests may print skips).

- [ ] **Step 5: Commit**

```bash
git add docs/images .serena/memories/documentation_captures.md rust/STATUS.md
git commit -m "docs(readme): recapture the media without the overlay"
```

No push.

---

## Final check

- [ ] `git status --short` shows only the camera FOV fix as uncommitted (parked, unchanged).
- [ ] Every still has `100` on all four border strips.
- [ ] The screensaver still shows the overlay (`show_overlay` defaults to `true`).
- [ ] `docs/previews/` is ignored and absent from the commit.
