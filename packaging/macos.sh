#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Nidvue
#
# Build the macOS .app bundle and .dmg. Signing/notarisation is opt-in: without
# the Apple identity the artifact is unsigned and the release notes say so.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
DIST="$ROOT/dist"
APP="$DIST/Nidvue Drive.app"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cargo build --release -p nidvue-cli -p nidvue-ffi --manifest-path "$ROOT/Cargo.toml"
install -m 0755 "$ROOT/target/release/nidvue-cli" "$APP/Contents/MacOS/nidvue-cli"
if [ -x "$ROOT/apps/nidvue-ui/build/nidvue-ui" ]; then
    install -m 0755 "$ROOT/apps/nidvue-ui/build/nidvue-ui" "$APP/Contents/MacOS/nidvue-ui"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Nidvue Drive</string>
  <key>CFBundleIdentifier</key><string>com.nidvue.drive</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>nidvue-ui</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHumanReadableCopyright</key><string>AGPL-3.0-only. Unofficial Jottacloud client.</string>
</dict></plist>
PLIST

cp "$ROOT/LICENSE" "$ROOT/NOTICE" "$APP/Contents/Resources/"
cat > "$APP/Contents/Resources/SOURCE-OFFER.txt" <<OFFER
Corresponding source: https://github.com/Nidvue/Drive/tree/v${VERSION}
OFFER

if command -v codesign >/dev/null && [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
    codesign --force --deep --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$APP"
else
    echo "no APPLE_SIGNING_IDENTITY: shipping unsigned"
fi

hdiutil create -volname "Nidvue Drive" -srcfolder "$APP" -ov -format UDZO \
    "$DIST/nidvue-drive-${VERSION}-macos.dmg"
(cd "$DIST" && shasum -a 256 ./*.dmg > SHA256SUMS)
echo "macos artifacts in $DIST"
