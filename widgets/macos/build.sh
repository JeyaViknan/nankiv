#!/usr/bin/env bash
# Builds NankivWidget.appex for bundling into nankiv.app.
#
#   widgets/macos/build.sh [output-directory]
#
# Universal by default, so one build serves both the Apple Silicon and Intel
# releases. Signed ad hoc, like the app; docs/RELEASING.md covers signing both
# with a Developer ID.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${1:-$here/build}"
version="$(/usr/bin/python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$here/../../src-tauri/tauri.conf.json")"

xcodebuild \
  -project "$here/NankivWidget.xcodeproj" \
  -scheme NankivWidget \
  -configuration Release \
  -derivedDataPath "$here/.build" \
  ARCHS="arm64 x86_64" \
  ONLY_ACTIVE_ARCH=NO \
  MARKETING_VERSION="$version" \
  build \
  -quiet

appex="$here/.build/Build/Products/Release/NankivWidget.appex"
mkdir -p "$out"
rm -rf "$out/NankivWidget.appex"
cp -R "$appex" "$out/"

# A widget that is not sandboxed, or not signed, will not load.
codesign --verify --strict "$out/NankivWidget.appex"
codesign -d --entitlements - "$out/NankivWidget.appex" 2>/dev/null | grep -q app-sandbox
echo "built $out/NankivWidget.appex"
