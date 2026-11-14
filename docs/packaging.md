# Packaging

| Platform | Format | Recipe | Mount helper |
|---|---|---|---|
| Linux | AppImage + `.deb` | `packaging/linux.sh` | none: unprivileged FUSE3 (`fusermount3`, `user_allow_other` only for `allow_other`) |
| macOS | `.dmg` | `packaging/macos.sh` | none: macFUSE system extension installed by the user |
| Windows | `.msi` | `packaging/windows.ps1` | none: WinFSP service installed as a prerequisite |

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

## macOS

- `.dmg` containing `Nidvue Drive.app` and a symlink to `/Applications`.
- macFUSE is a prerequisite installed by the user; the app detects its absence
  and explains it rather than failing with `ENODEV`.
- Hardened runtime and notarisation hooks are documented in the script; they
  require an Apple Developer identity and are skipped when it is absent, so CI
  passes without secrets.

## Windows

- MSI built with the WiX toolset; WinFSP is a prerequisite (GPL-3.0 with FLOSS
  exception — see `NOTICE`) and the MSI offers to install it.
- The mount runs in-process through WinFSP's user-mode filesystem API; no driver
  is bundled by this project.

## Signing

Signing hooks exist for all three platforms and are opt-in: without the relevant
secret the artifact is produced unsigned and the release notes say so. Signing
never gates a release build.
