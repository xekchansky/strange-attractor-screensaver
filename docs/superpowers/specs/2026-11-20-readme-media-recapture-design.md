# README media recapture — design

Date: 2026-11-20
Status: approved by the human partner

## Intent

The README screenshots and the hero GIF no longer match the Swift-era assets:
the formula overlay is burned into every frame and the Aizawa figure is cropped
by the frame edge. The goal is to restore the look of the assets shipped at tag
`swift-oracle` (no overlay, Swift-era framing) and, only where an exact match is
impossible, retune that attractor on its own.

## Findings that shaped the design

- The Swift capture rendered through an offscreen `SCNRenderer`; the overlay was
  an `NSTextField` outside that render, so it never reached the PNG. In the Rust
  port the overlay is drawn into the frame by `FrameRenderer::render`, and the
  CLI always passes it.
- The Swift camera measured its 60-degree field of view along the larger frame
  dimension (46.8 degrees vertically at 4:3). The port copied that, and the
  committed captures match the Swift stills exactly apart from the overlay.
- A CPU-only projection probe of the documented Aizawa view gives `max|y| = 1.19`
  under the Swift projection (19% cropped) and `0.89` under a vertical-60-degree
  projection. The same probe gives Lorenz `0.79` vs `0.595`: the vertical-60 fix
  shrinks every figure by 25%, which is not the Swift look.
- The Swift-era Aizawa still was captured without pinned parameters (the old
  script has no `aizawa` screenshot case), so it is not reproducible; the Rust
  script pins the canonical chaotic parameters, which is why that trace is
  denser and reaches the frame edge.

## Approaches considered

- **A. swift-era projection, per-attractor retune (chosen).** Keep SceneKit's
  FOV behaviour for capture so eleven assets match the tag one-to-one; retune
  only the attractors that still clip.
- **B. vertical-60-degree projection everywhere.** Nothing clips, but every
  figure is 25% smaller than the reference the human partner liked.
- **C. adaptive framing from the projected extent.** Always tight and unclipped,
  but it is a new composition, not a restoration.

The uncommitted camera fix on `fix/camera-framing` (vertical field of view) is a
separate product concern for 16:9 and the browser demo. It is stashed for the
capture and restored afterwards; it is not part of this iteration.

## Goals

- README stills and the hero GIF carry no overlay and match `swift-oracle`.
- A local preview folder and a side-by-side sheet let the human partner accept
  or reject each asset before anything lands in `docs/images`.
- `docs/images` and the published READMEs change only after approval.

## Non-goals

- Changing the screensaver's live overlay.
- Merging the camera FOV fix.
- Retuning attractors that already match the reference.

## Design

1. `PlayerOptions` gains `show_overlay: bool` (default `true`); `Player::render`
   and `render_to_surface` pass `None` when it is false. The CLI gains
   `--no-overlay`. Every existing shell keeps the overlay.
2. `docs/capture_all.sh` passes `--no-overlay` and writes to the output
   directory it is given; the preview convention is `docs/previews/<label>/`,
   which is git-ignored.
3. After the capture the human partner's GPU run produces the PNGs; the agent
   assembles the GIF with ffmpeg and writes `docs/previews/<label>/index.html`
   showing the Swift original next to the new frame, plus an edge-ink check that
   flags a clipped frame objectively.
4. Outliers get a targeted recipe (angle, trail length or start state) and a
   re-run of only that attractor via `ATTRACTORS=...`.
5. On approval the chosen files are copied into `docs/images`, the captures
   memory and `rust/STATUS.md` are updated, and one atomic commit is made. No
   push.

## Left to the builder

- Exact HTML layout of the comparison sheet.
- Whether the edge-ink threshold is per-mille of border pixels; any sane report
  that flags Aizawa and clears the other eleven is enough.
- The retune recipe for Aizawa.
