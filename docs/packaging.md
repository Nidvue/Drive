# Packaging

Related: [Release](Team/Release.md) · [deviations](deviations.md) · [configuration](configuration.md)

| Platform | Format | Recipe | Mount helper |
|---|---|---|---|
| Linux | AppImage + `.deb` | `packaging/linux.sh` | none: unprivileged FUSE3 (`fusermount3`, `user_allow_other` only for `allow_other`) |

Every artifact ships with:

- `LICENSE` (AGPL-3.0 text) and `NOTICE`;
- `SOURCE-OFFER.txt` naming the exact tag whose source corresponds to the binary
  (AGPL-3.0 §4-6), plus the source archive attached to the release;
- `SHA256SUMS` over every artifact.

## Linux

- AppImage: the `nidvue-cli` and `nidvue-ui` binaries, the Qt libraries, and a
  `.desktop` entry. Qt stays dynamically linked inside the AppImage (the AppImage
  runtime is the loader), so users can still substitute Qt builds.
- `.deb`: `nidvue-drive` depends on `libfuse3-3`, `libqt6quick6`,
  `libqt6qml6`, `fuse3`; the package does **not** install a setuid helper. The
  mount runs as the user through the kernel's FUSE device.
- The `.deb` installs a `nidvue-drive` systemd **user** unit (not system) for
  optional auto-mount.

## Login entry

`nidvue-cli autostart enable` writes
`$XDG_CONFIG_HOME/autostart/nidvue-drive.desktop` (`~/.config/autostart` when
`XDG_CONFIG_HOME` is unset), pointing at the application binary. The Settings
window's **Start at login** toggle writes the same file through the C ABI, so
either front door is enough. `autostart disable` removes it; `autostart status
--json` reports the state, the path and the command.

The `.deb` does not add a second login entry: a desktop that reads XDG autostart
and a systemd user unit would otherwise start the app twice.

## Signing

Signing hooks are opt-in and Linux-only in this release: without the relevant
secret the artifact is produced unsigned and the release notes say so. Signing
never gates a release build.
