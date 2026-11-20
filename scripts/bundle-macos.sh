#!/usr/bin/env bash
# Build a Finder-launchable app with the SigNexora name and icon.
# Usage: scripts/bundle-macos.sh [--release]
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script builds a macOS application bundle." >&2
  exit 1
fi
profile=debug
build_args=(-p sn-gui)
if [[ "${1:-}" == "--release" && "$#" == 1 ]]; then
  profile=release
  build_args+=(--release)
elif [[ "$#" != 0 ]]; then
  echo "Usage: scripts/bundle-macos.sh [--release]" >&2
  exit 1
fi
cargo build "${build_args[@]}"
target_dir="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
package_id="$(cargo pkgid -p sn-gui)"
version="${package_id##*#}"
version="${version##*@}"
bundle="$target_dir/$profile/SigNexora.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$target_dir/$profile/signexora" "$bundle/Contents/MacOS/signexora"
cp crates/sn-gui/assets/branding/signexora.icns "$bundle/Contents/Resources/signexora.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>SigNexora</string>
  <key>CFBundleDisplayName</key><string>SigNexora</string>
  <key>CFBundleIdentifier</key><string>app.signexora.desktop</string>
  <key>CFBundleExecutable</key><string>signexora</string>
  <key>CFBundleIconFile</key><string>signexora.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf 'APPL????' > "$bundle/Contents/PkgInfo"
plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
echo "Built $bundle"
