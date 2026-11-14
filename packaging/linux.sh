#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Nidvue
#
# Build the Linux artifacts: a .deb and a relocatable tar.gz.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
NAME="nidvue-drive_${VERSION}_${ARCH}"
DIST="$ROOT/dist"
STAGE="$DIST/$NAME"

rm -rf "$STAGE"
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/share/doc/nidvue-drive" "$STAGE/usr/lib/systemd/user" \
         "$STAGE/usr/share/applications" "$STAGE/usr/share/icons/hicolor/scalable/apps"

cargo build --release -p nidvue-cli -p nidvue-ffi --manifest-path "$ROOT/Cargo.toml"
install -m 0755 "$ROOT/target/release/nidvue-cli" "$STAGE/usr/bin/nidvue-cli"
if [ -x "$ROOT/apps/nidvue-ui/build/nidvue-ui" ]; then
    install -m 0755 "$ROOT/apps/nidvue-ui/build/nidvue-ui" "$STAGE/usr/bin/nidvue-ui"
fi

cp "$ROOT/LICENSE" "$ROOT/NOTICE" "$STAGE/usr/share/doc/nidvue-drive/"
cat > "$STAGE/usr/share/doc/nidvue-drive/SOURCE-OFFER.txt" <<OFFER
Corresponding source for Nidvue Drive ${VERSION}:
https://github.com/Nidvue/nidvue-drive/tree/v${VERSION}
AGPL-3.0 sections 4-6 are satisfied by the tag above and by the source archive
attached to the release.
OFFER

cat > "$STAGE/usr/share/applications/nidvue-drive.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Nidvue Drive
Comment=Unofficial Jottacloud sync and mount client
Exec=nidvue-ui
Icon=nidvue-drive
Categories=Network;FileTransfer;Utility;
DESKTOP

cat > "$STAGE/usr/lib/systemd/user/nidvue-drive.service" <<'UNIT'
[Unit]
Description=Nidvue Drive (unofficial Jottacloud client)
[Service]
ExecStart=/usr/bin/nidvue-cli mount
Restart=on-failure
[Install]
WantedBy=default.target
UNIT

mkdir -p "$STAGE/DEBIAN"
SIZE="$(du -sk "$STAGE" | cut -f1)"
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: nidvue-drive
Version: ${VERSION}
Section: net
Priority: optional
Architecture: ${ARCH}
Maintainer: Nidvue <nidvue@example.invalid>
Depends: libfuse3-3 | fuse3, libc6
Recommends: libqt6quick6, libqt6qml6
Homepage: https://github.com/Nidvue/nidvue-drive
Description: Unofficial Jottacloud desktop client
 Two-way sync and a mountable filesystem for Jottacloud: Rust engine, Qt UI,
 plugin system. This is an unofficial client, not affiliated with Jottacloud AS.
Installed-Size: ${SIZE}
CONTROL

dpkg-deb --root-owner-group --build "$STAGE" "$DIST/nidvue-drive_${VERSION}_${ARCH}.deb"

# A relocatable tarball for people who do not want a package manager.
TARNAME="nidvue-drive-${VERSION}-linux-${ARCH}"
mkdir -p "$DIST/$TARNAME"
cp -r "$STAGE/usr" "$DIST/$TARNAME/"
cp "$ROOT/LICENSE" "$ROOT/NOTICE" "$DIST/$TARNAME/"
cp "$STAGE/usr/share/doc/nidvue-drive/SOURCE-OFFER.txt" "$DIST/$TARNAME/"
tar -C "$DIST" -czf "$DIST/${TARNAME}.tar.gz" "$TARNAME"
rm -rf "$STAGE" "$DIST/$TARNAME"

(cd "$DIST" && sha256sum ./*.deb ./*.tar.gz > SHA256SUMS)
echo "linux artifacts in $DIST"
