#!/bin/zsh
# Build the distributable screen saver bundle and zip it for a GitHub release.
#
# Usage: ./scripts/package_release.sh [output-directory]
#
# Produces, in the output directory (default: ./dist):
#   AttractorSaver.saver           the ad-hoc signed bundle
#   AttractorSaver.saver.zip       the file uploaded to the release
#   AttractorSaver.saver.zip.sha256
#
# Requires Xcode and xcodegen. The bundle is intentionally not notarized;
# see README.md for the Gatekeeper note shown to users.
set -euo pipefail

ROOT=${0:A:h:h}
OUT=${1:-"$ROOT/dist"}
mkdir -p "$OUT"

echo "==> Generating the Xcode project"
cd "$ROOT/Host"
xcodegen generate

echo "==> Building Release"
xcodebuild \
  -project AttractorHost.xcodeproj \
  -scheme AttractorSaver \
  -configuration Release \
  -derivedDataPath "$OUT/derivedData" \
  build

BUILT="$OUT/derivedData/Build/Products/Release/AttractorSaver.saver"
if [[ ! -d "$BUILT" ]]; then
  echo "error: $BUILT was not produced" >&2
  exit 1
fi

echo "==> Staging the bundle"
rm -rf "$OUT/AttractorSaver.saver" "$OUT/AttractorSaver.saver.zip"
ditto "$BUILT" "$OUT/AttractorSaver.saver"

# The build already signs ad-hoc; re-signing explicitly keeps the published
# bundle valid even if a local signing identity leaks into the build.
codesign --force --deep --sign - "$OUT/AttractorSaver.saver"

echo "==> Verifying the signature"
codesign --verify --deep --strict --verbose=2 "$OUT/AttractorSaver.saver"

echo "==> Zipping"
ditto -c -k --sequesterRsrc --keepParent \
  "$OUT/AttractorSaver.saver" "$OUT/AttractorSaver.saver.zip"

shasum -a 256 "$OUT/AttractorSaver.saver.zip" \
  | tee "$OUT/AttractorSaver.saver.zip.sha256"

echo "==> Done"
ls -lh "$OUT/AttractorSaver.saver.zip"
