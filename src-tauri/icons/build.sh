#!/usr/bin/env bash
# Builds every app icon from AppIcon.icon, the Icon Composer document.
#
#   npm run icon:build
#
# To change the icon, open AppIcon.icon in Icon Composer, edit, save, and run
# this. It writes:
#
#   Assets.car   the layered icon macOS 26 and later draw with Liquid Glass —
#                light, dark, tinted and clear — plus flat renderings older
#                versions of macOS use. Found through CFBundleIconName.
#   icon.icns    the last-resort fallback, compiled alongside it by actool.
#   *.png, .ico  Windows and Linux, from Apple's own rendering of the icon.
#
# The outputs are committed, so a build needs only the result: compiling an
# .icon needs Xcode 26 or later, which a release machine may not have.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

xcrun actool "$here/AppIcon.icon" \
  --compile "$work" \
  --app-icon AppIcon \
  --include-all-app-icons \
  --output-partial-info-plist "$work/partial.plist" \
  --platform macosx \
  --target-device mac \
  --minimum-deployment-target 10.15 \
  --development-region en \
  --enable-on-demand-resources NO \
  --errors --warnings --output-format human-readable-text >/dev/null

cp "$work/Assets.car" "$here/Assets.car"
cp "$work/AppIcon.icns" "$here/icon.icns"

# One large rendering, in the current design generation, for the platforms
# that take a bitmap. The renderer is the ictool inside Icon Composer — the one
# on Xcode's path of the same name is a different, older tool. The standalone
# Icon Composer is preferred: it is the newer of the two, and the copy inside
# Xcode 26.0 takes an older form of the command.
ictool=""
for app in "/Applications/Icon Composer.app" "$(xcode-select -p)/../Applications/Icon Composer.app"; do
  [ -x "$app/Contents/Executables/ictool" ] && ictool="$app/Contents/Executables/ictool" && break
done
[ -n "$ictool" ] || { echo "Icon Composer not found; install Xcode 26 or later" >&2; exit 1; }
"$ictool" "$here/AppIcon.icon" --export-image \
  --output-file "$work/AppIcon.png" \
  --platform macOS --rendition Default \
  --width 1024 --height 1024 --scale 1 >/dev/null 2>&1 ||
  "$ictool" "$here/AppIcon.icon" --export-preview macOS Light 1024 1024 1 "$work/AppIcon.png"
npx --no-install tauri icon "$work/AppIcon.png" --output "$work/generated" >/dev/null
for f in 32x32.png 128x128.png 128x128@2x.png icon.png icon.ico; do
  cp "$work/generated/$f" "$here/$f"
done

echo "icons written to $here"
