#!/usr/bin/env bash
# Export platform icon sizes from the generated PNG master (macOS tools).
set -euo pipefail
cd "$(dirname "$0")/.."

assets="crates/sn-gui/assets/branding"
icon_work="$(mktemp -d "${TMPDIR:-/tmp}/signexora-icons.XXXXXX")"
trap 'rm -rf "$icon_work"' EXIT
mkdir -p "$icon_work/signexora.iconset"

for side in 16 32 48 64 128 256 512 1024; do
  sips -z "$side" "$side" "$assets/signexora.png" --out "$icon_work/$side.png" >/dev/null
done
cp "$icon_work/64.png" "$assets/signexora-64.png"
for side in 16 32 128 256 512; do
  cp "$icon_work/$side.png" "$icon_work/signexora.iconset/icon_${side}x${side}.png"
  cp "$icon_work/$((side * 2)).png" "$icon_work/signexora.iconset/icon_${side}x${side}@2x.png"
done
iconutil -c icns "$icon_work/signexora.iconset" -o "$assets/signexora.icns"

# Package the exported PNGs into an ICO container; no image processing here.
python3 - "$icon_work" "$assets/signexora.ico" <<'PY'
from pathlib import Path
import struct
import sys

source, destination = map(Path, sys.argv[1:])
sizes = (16, 32, 48, 64, 128, 256)
offset = 6 + 16 * len(sizes)
entries, images = [], []
for side in sizes:
    image = (source / f'{side}.png').read_bytes()
    entries.append(struct.pack('<BBBBHHII', side % 256, side % 256, 0, 0, 1, 32, len(image), offset))
    images.append(image)
    offset += len(image)
destination.write_bytes(struct.pack('<HHH', 0, 1, len(sizes)) + b''.join(entries) + b''.join(images))
PY
echo "Exported SigNexora PNG, ICNS and ICO icons."
