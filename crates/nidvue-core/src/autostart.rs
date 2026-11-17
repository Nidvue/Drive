// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! XDG autostart: the `.desktop` file that starts the app when the user logs in.
//!
//! Linux desktops read `$XDG_CONFIG_HOME/autostart/*.desktop` at session start,
//! so this is the portable way to offer "start at login" without a per-desktop
//! API. The file is a plain record: `enable` writes it, `disable` removes it,
//! and `enabled` is its existence.

use std::path::{Path, PathBuf};

use crate::error::{NidvueError, Result};

/// The file name a desktop looks for.
pub const FILE_NAME: &str = "nidvue-drive.desktop";

/// The autostart file for the current user.
pub fn path() -> PathBuf {
    path_in(&base_dir())
}

/// The autostart file under an explicit configuration directory.
pub fn path_in(base: &Path) -> PathBuf {
    base.join("autostart").join(FILE_NAME)
}

/// Write the autostart file, starting `exec` at login. Idempotent.
pub fn enable(exec: &str) -> Result<PathBuf> {
    enable_in(&base_dir(), exec)
}

/// Write the autostart file under `base`, starting `exec` at login.
pub fn enable_in(base: &Path, exec: &str) -> Result<PathBuf> {
    let exec = exec.trim();
    if exec.is_empty() {
        return Err(NidvueError::Config(
            "autostart needs the command to start".into(),
        ));
    }
    let file = path_in(base);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| NidvueError::io(dir, e))?;
    }
    std::fs::write(&file, entry(exec)).map_err(|e| NidvueError::io(&file, e))?;
    Ok(file)
}

/// Remove the autostart file. Returns true when one was there.
pub fn disable() -> Result<bool> {
    disable_in(&base_dir())
}

/// Remove the autostart file under `base`.
pub fn disable_in(base: &Path) -> Result<bool> {
    let file = path_in(base);
    match std::fs::remove_file(&file) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(NidvueError::io(&file, e)),
    }
}

/// True when the current user starts the app at login.
pub fn enabled() -> bool {
    enabled_in(&base_dir())
}

/// True when `base` has an autostart file.
pub fn enabled_in(base: &Path) -> bool {
    path_in(base).is_file()
}

/// The `Exec=` line of the current user's autostart file, when it exists.
pub fn exec() -> Option<String> {
    exec_in(&base_dir())
}

/// The `Exec=` line under `base`.
pub fn exec_in(base: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path_in(base)).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("Exec="))
        .map(str::to_string)
}

fn entry(exec: &str) -> String {
    // The desktop-entry grammar splits Exec on whitespace unless the path is
    // quoted, so a path with a space in it has to be quoted.
    let exec = if exec.contains(' ') && !exec.starts_with('"') {
        format!("\"{exec}\"")
    } else {
        exec.to_string()
    };
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Nidvue Drive\n\
         Comment=Unofficial Jottacloud sync and mount client\n\
         Exec={exec}\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    )
}

fn base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_writes_a_desktop_entry_and_reads_it_back() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!enabled_in(dir.path()));
        let file = enable_in(dir.path(), "/usr/bin/nidvue-ui").unwrap();
        assert_eq!(file, path_in(dir.path()));
        assert!(enabled_in(dir.path()));
        assert_eq!(exec_in(dir.path()).as_deref(), Some("/usr/bin/nidvue-ui"));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("[Desktop Entry]\n"));
        assert!(text.contains("X-GNOME-Autostart-enabled=true"));
    }

    #[test]
    fn enable_is_idempotent_and_disable_reports_what_it_did() {
        let dir = tempfile::tempdir().unwrap();
        enable_in(dir.path(), "/usr/bin/nidvue-ui").unwrap();
        enable_in(dir.path(), "/usr/bin/nidvue-ui").unwrap();
        assert!(disable_in(dir.path()).unwrap());
        assert!(!enabled_in(dir.path()));
        assert!(
            !disable_in(dir.path()).unwrap(),
            "a second disable removes nothing"
        );
    }

    #[test]
    fn a_path_with_a_space_is_quoted_and_an_empty_command_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        enable_in(dir.path(), "/opt/my apps/nidvue-ui").unwrap();
        assert_eq!(
            exec_in(dir.path()).as_deref(),
            Some("\"/opt/my apps/nidvue-ui\"")
        );
        assert!(enable_in(dir.path(), "   ").is_err());
    }

    #[test]
    fn the_path_follows_xdg_config_home() {
        assert_eq!(
            path_in(Path::new("/home/u/.config")),
            Path::new("/home/u/.config/autostart/nidvue-drive.desktop")
        );
    }
}
