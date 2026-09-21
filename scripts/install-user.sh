#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${SHADOW_BATCH_BIN:-$ROOT/target/release/shadow-batch-processor}"
if [[ ! -x "$BIN" ]]; then
  (cd "$ROOT" && cargo build --release)
  BIN="$ROOT/target/release/shadow-batch-processor"
fi
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
install -D -m 0755 "$BIN" "$HOME/.local/bin/shadow-batch-processor"
install -D -m 0644 "$ROOT/data/com.shadowfetch.BatchProcessor.desktop" \
  "$PREFIX/applications/com.shadowfetch.BatchProcessor.desktop"
install -D -m 0644 "$ROOT/data/icons/hicolor/scalable/apps/shadow-batch-processor.svg" \
  "$PREFIX/icons/hicolor/scalable/apps/shadow-batch-processor.svg"
for size in 16 24 32 48 64 128 256 512; do
  install -D -m 0644 "$ROOT/data/icons/hicolor/${size}x${size}/apps/shadow-batch-processor.png" \
    "$PREFIX/icons/hicolor/${size}x${size}/apps/shadow-batch-processor.png"
done
update-desktop-database "$PREFIX/applications" || true
gtk-update-icon-cache -f -t "$PREFIX/icons/hicolor" >/dev/null 2>&1 || true
desktop-file-validate "$PREFIX/applications/com.shadowfetch.BatchProcessor.desktop"
echo "Installed $HOME/.local/bin/shadow-batch-processor"
