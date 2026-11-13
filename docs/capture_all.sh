#!/bin/zsh
# Capture screenshots of all catalog attractors + Aizawa frames for the hero GIF.
# Requires a GUI session (WindowServer): capture needs Metal, so run this from
# a normal logged-in terminal, not an SSH/background session.
#
# Usage: ./docs/capture_all.sh <binary-path> <output-dir>
#
# Build the binary first:  cargo build -p attractor-cli
# Then:                    ./docs/capture_all.sh rust/target/debug/attractor-cli "$TMPDIR/captures"
set -e
BIN="${1:?Usage: capture_all.sh <binary-path> <output-dir>}"
OUT="${2:?Usage: capture_all.sh <binary-path> <output-dir>}"
mkdir -p "$OUT/screenshots" "$OUT/aizawa-frames"

# Override with e.g. ATTRACTORS="thomas chua" ./docs/capture_all.sh <bin> <out>
# to re-capture only the failed set.
ATTRACTORS=${ATTRACTORS:-lorenz rossler thomas chen aizawa lu chua sprott_a sprott_b sprott_c halvorsen moore_spiegel}

# The seed pins the values a case does not spell out (unpinned parameters,
# hyperparameters and the initial state), so the documentation baseline is
# reproducible rather than a different picture every time.
SEED=${SEED:-42}

capture_args() {
  case "$1" in
    # The Sprott family's 2000–8000 default leaves the figure sparse here.
    sprott_*) echo "--trail-length 20000" ;;
    # Random Aizawa sampling lands in near-periodic pockets at many seeds
    # (42, 43 and 7 all fail the health check), so the documentation view pins
    # the canonical chaotic parameters the hero recording uses.
    aizawa)
      echo "-p a=0.95 -p b=0.7 -p c=0.6 -p d=3.5 -p e=0.25 -p f=0.1 --step 0.002 --speed 0.7 --cam-azimuth 45 --cam-elevation 36 --cam-speed 0 --trail-color-speed 0" ;;
    # Pinned documentation views. Unpinned runs sample parameters and a random
    # camera azimuth on every launch, which makes a screenshot impossible to
    # reproduce and often lands on an unflattering side. Each set below was
    # checked to stay healthy over the whole warm-up (RK4, `SimulationHealthMonitor`).
    # `--cam-speed 0` keeps the pinned azimuth exact: capture settles for 90
    # ticks before the snapshot and the orbit would otherwise drift ~20 degrees.
    # `--trail-color-speed 0` freezes the gradient offset for the same reason.
    # Rössler is a flat spiral in x-y with a tall lift along z, so the two are
    # perpendicular: no single view shows both face-on. The lift only stands
    # upright with the orbit on the -z side (azimuth 270), and 85 degrees tilts
    # it 5 degrees off vertical, which is the most upright the spiral survives.
    rossler)
      echo "-p a=0.2 -p b=0.2 -p c=5.7 --cam-azimuth 270 --cam-elevation 85 --cam-speed 0 --trail-color-speed 0" ;;
    # Thomas is the one attractor whose catalog midpoint start collapses into a
    # fixed point for most of its `b` range, so the capture also pins a start
    # inside the chaotic basin. b=0.19 fills the whole [-4,4] cube (at b=0.208
    # the web stays confined between z 0 and 4). Views near the (1,1,1) symmetry
    # axis read as a flat rosette with a hollow centre; azimuth 0 at a 10 degree
    # elevation looks at the woven shell from the side, where it has depth.
    thomas)
      echo "-p b=0.19 --initial-state 0.6,0.6,-0.86 --trail-length 20000 --cam-azimuth 0 --cam-elevation 10 --cam-speed 0 --trail-color-speed 0" ;;
    chen)
      echo "-p a=38 -p b=3 -p c=28 --cam-azimuth 120 --cam-elevation 25 --cam-speed 0 --trail-color-speed 0" ;;
    # Lü's two lobes separate along (1,1,0), so the default azimuth 45 looks
    # straight down their axis and flattens the butterfly; 20 degrees off adds
    # the side view without losing the shape.
    lu)
      echo "-p a=36 -p b=3 -p c=20 --cam-azimuth 20 --cam-elevation 20 --cam-speed 0 --trail-color-speed 0" ;;
    # Chua's double scroll is stiff: at step 0.0006 a 2000–8000 point trail only
    # covers a fraction of the scroll, so the figure looks unfinished — one well
    # stays half-formed. 24000 points and a longer warm-up trace both scrolls.
    # Face-on (elevation 85) the two scrolls collapse into a rounded rectangle,
    # and a strong tilt (45 degrees) reads as flat sheets; 65 keeps both spirals
    # distinct with enough depth.
    chua)
      echo "-p alpha=15.6 -p beta=25.6 -p m0=-1.143 -p m1=-0.714 --trail-length 24000 --warmup-steps 200000 --cam-azimuth 90 --cam-elevation 65 --cam-speed 0 --trail-color-speed 0" ;;
    # Moore–Spiegel's two spiral centres sit at x = ±1.6. Looking along x (the
    # previous azimuth 30) puts one spiral in the foreground at full size and
    # the other tiny behind it; a view along z at a low elevation shows both
    # side by side at nearly the same distance. The 5000-point default window
    # also favours one lobe, so the trail is longer: 20000 points square both.
    # A finer step (0.003 instead of the 0.007 midpoint) shortens the chords
    # between trail points: the fast Moore–Spiegel loops looked like broken
    # straight segments at 0.007.
    moore_spiegel)
      echo "-p R=250 -p T=10 --step 0.003 --trail-length 20000 --cam-azimuth 90 --cam-elevation 12 --cam-speed 0 --trail-color-speed 0" ;;
    *) echo "" ;;
  esac
}

capture_with_retry() {
  local id="$1"
  local log attempt seed
  shift
  for attempt in 1 2 3 4 5; do
    # A fixed seed makes a degenerate run reproducible too, so every retry
    # advances the seed instead of replaying the same failure. The first
    # attempt is the documented one; later seeds only appear when it failed.
    seed=$((SEED + attempt - 1))
    log=$(mktemp)
    # `if !` keeps a non-zero exit inside the tested command: with `set -e`
    # a bare "$BIN" line would abort the whole script on the first failed
    # (degenerate) run instead of retrying.
    if ! "$BIN" --attractor "$id" --seed "$seed" "$@" > "$log" 2>&1; then
      echo "    attempt $attempt (seed $seed) failed, retrying (bad run detected)"
      rm -f "$log"
    else
      rm -f "$log"
      return 0
    fi
  done
  echo "    giving up on $id" >&2
  return 1
}

echo "=== Screenshots of all attractors ==="
# `${=...}`: zsh does not word-split unquoted parameters, so the env override
# `ATTRACTORS="thomas chua"` would otherwise arrive as a single name.
for a in ${=ATTRACTORS}; do
  echo "--- $a"
  # `${=...}` re-splits the helper's output into separate words (zsh does not
  # word-split command substitution by default).
  capture_with_retry "$a" ${=$(capture_args "$a")} --screenshot "$OUT/screenshots/$a.png"
done

# The hero recording takes a couple of minutes; SKIP_HERO=1 replays only the
# catalog screenshots (handy when re-capturing a few attractors).
if [[ ${SKIP_HERO:-0} != 1 ]]; then
echo "=== Aizawa frames for the hero GIF ==="
# Canonical chaotic parameters, pinned explicitly: random sampling keeps
# landing in near-periodic pockets that read as a closed ring, and the
# DebugConfig midpoint step (0.007) itself settles into a loop here — 0.008
# was verified healthy over 200k steps. --warmup-steps 0 shows the trail
# growing from the start. Capture pins the frame interval to 1/60 s, and a
# requested second is 60 rendered ticks, so every captured frame advances the
# same amount of simulated time. 40 s of recording at 5.83 steps per frame
# fills the Aizawa trail (14000 points) exactly by the last frame, so the
# figure is fully traced when the animation ends; the 2400 captured frames
# play back as 48 s at the GIF's 50 fps.
for attempt in 1 2 3 4 5; do
  if "$BIN" --attractor aizawa --seed "$SEED" \
      -p a=0.95 -p b=0.7 -p c=0.6 -p d=3.5 -p e=0.25 -p f=0.1 \
      --step 0.002 --speed 0.7 --warmup-steps 0 \
      --cam-speed 0.04 --trail-color-speed 0.05 --capture-scale 1 \
      --record "$OUT/aizawa-frames" --record-seconds 40 --frame-stride 1; then
    break
  fi
  echo "    record attempt $attempt failed, retrying"
done
fi

echo "=== Done ==="
ls -la "$OUT/screenshots" | head -20
echo "total frames: $(ls "$OUT/aizawa-frames" | wc -l)"

# Assemble the hero animation from the recorded frames (needs ffmpeg with
# palettegen/paletteuse). The asset is a GIF, not an animated WebP: WebP
# animation is encoded as 4:2:0 with inter-frame deltas, and on this footage
# that left pale vertical smear bands over the trail, plus GitHub does not
# animate WebP in READMEs. The GIF is cleaner (palette dithering only) and
# smaller. 640px, 50 fps (2 cs delay, the shortest delay browsers honour) and
# 2400 frames give a 48 s loop.
#   mkdir -p "$OUT/aizawa-640"
#   for f in "$OUT"/aizawa-frames/frame_*.png; do
#     sips -Z 640 "$f" --out "$OUT/aizawa-640/$(basename "$f")"
#   done
#   ffmpeg -y -framerate 50 -i "$OUT/aizawa-640/frame_%05d.png" \
#     -vf "palettegen=max_colors=256:stats_mode=full" -update 1 "$OUT/pal.png"
#   ffmpeg -y -framerate 50 -i "$OUT/aizawa-640/frame_%05d.png" -i "$OUT/pal.png" \
#     -lavfi "paletteuse=dither=bayer:bayer_scale=3" \
#     docs/images/strange-attractor-aizawa-screensaver-animation.gif
