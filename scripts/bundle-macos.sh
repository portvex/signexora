#!/usr/bin/env bash
# Build the release Finder app: <target>/release/SigNexora.app
# Usage: scripts/bundle-macos.sh
# `--release` is accepted so older commands keep working. There is no debug bundle.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script builds a macOS application bundle." >&2
  exit 1
fi
if [[ "$#" -gt 1 || ( "$#" == 1 && "$1" != "--release" ) ]]; then
  echo "Usage: scripts/bundle-macos.sh" >&2
  exit 1
fi
cargo build --release -p sn-gui
target_dir="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
package_id="$(cargo pkgid -p sn-gui)"
version="${package_id##*#}"
version="${version##*@}"
bundle="$target_dir/release/SigNexora.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$target_dir/release/signexora" "$bundle/Contents/MacOS/signexora"
cp crates/sn-gui/assets/branding/signexora.icns "$bundle/Contents/Resources/signexora.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>SigNexora</string>
  <key>CFBundleDisplayName</key><string>SigNexora</string>
  <key>CFBundleIdentifier</key><string>app.signexora.desktop</string>
  <key>CFBundleExecutable</key><string>signexora</string>
  <key>CFBundleIconFile</key><string>signexora</string>
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
