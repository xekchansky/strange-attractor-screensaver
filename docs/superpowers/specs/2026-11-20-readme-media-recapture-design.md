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
- The port assumed SceneKit measured its 60-degree field of view along the
  larger frame dimension (46.8 degrees vertically at 4:3), and a CPU-only
  projection probe supported that reading. **The probe was falsified by
  measurement:** the captures rendered with the horizontal-60 behaviour are
  exactly 1.333x larger than the `swift-oracle` stills (ink bboxes: Rössler
  1380 vs 1030 px, Chen 1408 vs 1054, Lorenz 1530 vs 1096), and
  `tan 30° / tan 23.4° = 1.333`. The Swift behaviour is the **vertical** 60
  degrees, so the camera fix on `fix/camera-framing` is a parity fix, not a new
  composition; it is part of this iteration. See the revision note below.
- The CPU probe of the documented Aizawa view still gave `max|y| = 1.19` under
  horizontal-60 and `0.89` under vertical-60, which is why the cropped frames
  and the 25%-smaller estimate appeared in the first draft of this spec.
- The Swift-era Aizawa still was captured without pinned parameters (the old
  script has no `aizawa` screenshot case), so it is not reproducible; the Rust
  script pinned the canonical chaotic parameters, which is why that trace was
  denser and reached the frame edge.

## Approaches considered

- **A. horizontal-60 projection, per-attractor retune (first draft).** Assumed
  SceneKit's FOV along the larger dimension and planned to keep it; abandoned
  once the ink-bbox measurement showed horizontal-60 renders are 1.333x larger
  than the tag's stills.
- **B. vertical-60-degree projection (chosen).** The measured Swift behaviour:
  the stills match the tag almost pixel for pixel and nothing clips. The
  camera fix from `fix/camera-framing` is therefore promoted from "separate
  product concern" to part of this iteration.
- **C. adaptive framing from the projected extent.** Always tight and unclipped,
  but it is a new composition, not a restoration.

The camera fix is committed in this branch (`fd5f1dd`) as the parity fix
described above.

## Goals

- README stills and the hero GIF carry no overlay and match `swift-oracle`.
- A local preview folder and a side-by-side sheet let the human partner accept
  or reject each asset before anything lands in `docs/images`.
- `docs/images` and the published READMEs change only after approval.

## Non-goals

- Changing the screensaver's live overlay.
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

## Revision 2026-11-20 (during implementation)

The pre-flight probe this spec's first draft relied on was wrong about which
axis SceneKit measures its field of view along. Measuring the committed captures
against the `swift-oracle` stills showed the horizontal-60 renders are exactly
1.333x larger (`tan 30° / tan 23.4°`), so the vertical-60 camera fix is the Swift
parity behaviour, not a new composition: it was committed in this branch
(`fd5f1dd`) instead of being parked, and the Findings, Approaches and Non-goals
sections above are amended accordingly.

Two further deviations from the first draft: the delivery is a sequence of
commits on `fix/camera-framing` rather than one atomic commit, and Aizawa and
Halvorsen are retuned to the seed-picked random instances 28 and 50 (azimuths
315° and 185°) instead of the canonical parameters, because the Swift-era stills
for those two used unpinned random draws.
