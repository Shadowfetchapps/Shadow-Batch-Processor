#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)"
ARCH="$(dpkg --print-architecture)"
NAME="shadow-batch-processor_${VERSION}_${ARCH}.deb"
cd "$ROOT"
cargo build --release
STAGE="$ROOT/dist/deb-root"
rm -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" "$STAGE/usr/bin" "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/doc/shadow-batch-processor" "$STAGE/usr/share/icons/hicolor/scalable/apps"
install -m 0755 target/release/shadow-batch-processor "$STAGE/usr/bin/shadow-batch-processor"
install -m 0644 data/com.shadowfetch.BatchProcessor.desktop "$STAGE/usr/share/applications/"
install -m 0644 data/icons/hicolor/scalable/apps/shadow-batch-processor.svg \
  "$STAGE/usr/share/icons/hicolor/scalable/apps/"
for size in 16 24 32 48 64 128 256 512; do
  install -D -m 0644 "data/icons/hicolor/${size}x${size}/apps/shadow-batch-processor.png" \
    "$STAGE/usr/share/icons/hicolor/${size}x${size}/apps/shadow-batch-processor.png"
done
install -m 0644 README.md "$STAGE/usr/share/doc/shadow-batch-processor/"
install -m 0644 LICENSE "$STAGE/usr/share/doc/shadow-batch-processor/copyright"
SIZE="$(du -sk "$STAGE" | cut -f1)"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: shadow-batch-processor
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: Shadow Batch Processor contributors <209457103+ShadowfetchLinux@users.noreply.github.com>
Depends: ffmpeg, libgtk-4-1, libadwaita-1-0
Installed-Size: ${SIZE}
Homepage: https://github.com/ShadowfetchLinux/Shadow-Batch-Processor
Description: Bulk media pipelines for Linux
 Native GTK4 app that runs local rename/convert pipelines on many files.
EOF
mkdir -p "$ROOT/dist"
dpkg-deb --build "$STAGE" "$ROOT/dist/$NAME"
echo "Wrote $ROOT/dist/$NAME"
