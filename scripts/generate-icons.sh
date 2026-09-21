#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SVG="$ROOT/data/icons/hicolor/scalable/apps/shadow-batch-processor.svg"
for size in 16 24 32 48 64 128 256 512; do
  dir="$ROOT/data/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$dir"
  rsvg-convert -w "$size" -h "$size" -o "$dir/shadow-batch-processor.png" "$SVG"
done
echo "Generated batch icons"
