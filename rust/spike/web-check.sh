#!/usr/bin/env bash
# Headless Chrome runs for the wasm spike.
#
# The macOS sandbox cannot start a browser or reach the GPU, so this script is
# listed in .claude/settings.local.json under sandbox.excludedCommands. It
# exists only to make the I0.5 visual checks repeatable by an agent; it goes
# away with the rest of rust/spike in I6.
#
# Screenshot mode (captures once, then exits):
#   ./rust/spike/web-check.sh URL OUT.png [CHROME FLAGS...]
#
# Serve mode (keeps running for DevTools Protocol clients):
#   ./rust/spike/web-check.sh --serve URL [CHROME FLAGS...]
set -euo pipefail

chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
common=(
  --headless=new
  --no-first-run
  --no-default-browser-check
  --user-data-dir="${TMPDIR:-/tmp}/chrome-spike"
  --window-size=1280,720
)

if [[ "${1:-}" == "--serve" ]]; then
  shift
  url="${1:?usage: web-check.sh --serve URL [flags...]}"
  shift
  exec "$chrome" "${common[@]}" "$@" "$url"
fi

url="${1:?usage: web-check.sh URL OUT.png [flags...]}"
out="${2:?usage: web-check.sh URL OUT.png [flags...]}"
shift 2

exec "$chrome" "${common[@]}" --screenshot="$out" "$@" "$url"
