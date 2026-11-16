// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Credential storage.
//!
//! Tokens are never written in plaintext and never formatted into a log line.
//! The default store is a single file encrypted with ChaCha20-Poly1305 under a
//! random 256-bit key kept in a `0600` key file next to it (or supplied through
//! `NIDVUE_SECRET_KEY` as hex).
//!
//! Threat model, stated plainly: this protects against accidental disclosure -
//! config dumps, backups, log scraping, a crash report - not against an
//! attacker who can already read the state directory. The stronger store is the
//! OS keyring ([`KeyringStore`]), selected by `secret.backend`; see
//! `docs/deviations.md`.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::config::SecretBackend;
use crate::error::{NidvueError, Result};

const MAGIC: &[u8; 7] = b"NIDVUE1";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// A secret that does not print itself.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a secret.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Use the secret. Every call site is expected to be a request header or a
    /// keyring write, never a log line.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// A safe representation for logs: length only.
    pub fn redacted(&self) -> String {
        format!("<redacted {} bytes>", self.0.len())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.redacted())
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// A place tokens can live.
pub trait SecretStore: Send + Sync {
    /// Read a secret.
    fn load(&self, key: &str) -> Result<Option<SecretString>>;
    /// Write or replace a secret.
    fn store(&self, key: &str, value: &SecretString) -> Result<()>;
    /// Remove a secret; missing is not an error.
    fn delete(&self, key: &str) -> Result<()>;
}

/// The on-disk encrypted store.
pub struct EncryptedFileStore {
    path: PathBuf,
    key_path: PathBuf,
}

impl EncryptedFileStore {
    /// Open (or prepare) the store at `path`, with its key beside it.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let key_path = path.with_extension("key");
        Self { path, key_path }
    }

    /// Load the whole map, decrypting if the file exists.
    fn read_map(&self) -> Result<BTreeMap<String, String>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let raw = std::fs::read(&self.path).map_err(|e| NidvueError::io(&self.path, e))?;
        if raw.len() < MAGIC.len() + NONCE_LEN + 16 {
            return Err(NidvueError::Secret(format!(
                "{}: truncated",
                self.path.display()
            )));
        }
        if &raw[..MAGIC.len()] != MAGIC {
            return Err(NidvueError::Secret(format!(
                "{}: not a Nidvue secret file",
                self.path.display()
            )));
        }
        let key = self.load_key()?;
        let cipher = ChaCha20Poly1305::new(&Key::from(key));
        let nonce = Nonce::try_from(&raw[MAGIC.len()..MAGIC.len() + NONCE_LEN])
            .map_err(|_| NidvueError::Secret("nonce length".into()))?;
        let plain = cipher
            .decrypt(&nonce, &raw[MAGIC.len() + NONCE_LEN..])
            .map_err(|_| {
                NidvueError::Secret(format!("{}: decryption failed", self.path.display()))
            })?;
        serde_json::from_slice(&plain).map_err(|e| {
            NidvueError::Secret(format!("{}: corrupt payload: {e}", self.path.display()))
        })
    }

    /// Encrypt and atomically replace the map.
    fn write_map(&self, map: &BTreeMap<String, String>) -> Result<()> {
        let key = self.load_key()?;
        let plain =
            serde_json::to_vec(map).map_err(|e| NidvueError::Secret(format!("serialise: {e}")))?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce_bytes)
            .map_err(|e| NidvueError::Secret(format!("entropy: {e}")))?;
        let cipher = ChaCha20Poly1305::new(&Key::from(key));
        let nonce = Nonce::from(nonce_bytes);
        let ct = cipher
            .encrypt(&nonce, plain.as_slice())
            .map_err(|_| NidvueError::Secret("encryption failed".into()))?;
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        }
        let tmp = self.path.with_extension("enc.tmp");
        write_private(&tmp, &out)?;
        std::fs::rename(&tmp, &self.path).map_err(|e| NidvueError::io(&self.path, e))?;
        Ok(())
    }

    fn load_key(&self) -> Result<[u8; KEY_LEN]> {
        if let Ok(hex_key) = std::env::var("NIDVUE_SECRET_KEY") {
            let bytes = hex::decode(hex_key.trim())
                .map_err(|_| NidvueError::Secret("NIDVUE_SECRET_KEY is not hex".into()))?;
            let arr: [u8; KEY_LEN] = bytes.try_into().map_err(|_| {
                NidvueError::Secret(format!("NIDVUE_SECRET_KEY must be {KEY_LEN} bytes"))
            })?;
            return Ok(arr);
        }
        if self.key_path.exists() {
            let bytes =
                std::fs::read(&self.key_path).map_err(|e| NidvueError::io(&self.key_path, e))?;
            let arr: [u8; KEY_LEN] = bytes.as_slice().try_into().map_err(|_| {
                NidvueError::Secret(format!(
                    "{}: key must be {KEY_LEN} bytes",
                    self.key_path.display()
                ))
            })?;
            return Ok(arr);
        }
        let mut key = [0u8; KEY_LEN];
        getrandom::fill(&mut key).map_err(|e| NidvueError::Secret(format!("entropy: {e}")))?;
        if let Some(parent) = self.key_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| NidvueError::io(parent, e))?;
        }
        write_private(&self.key_path, &key)?;
        Ok(key)
    }
}

/// Write a file only the current user can read.
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).map_err(|e| NidvueError::io(path, e))?;
    f.write_all(bytes).map_err(|e| NidvueError::io(path, e))?;
    f.sync_all().map_err(|e| NidvueError::io(path, e))?;
    Ok(())
}

impl SecretStore for EncryptedFileStore {
    fn load(&self, key: &str) -> Result<Option<SecretString>> {
        Ok(self.read_map()?.remove(key).map(SecretString::new))
    }

    fn store(&self, key: &str, value: &SecretString) -> Result<()> {
        let mut map = self.read_map()?;
        map.insert(key.to_string(), value.expose().to_string());
        self.write_map(&map)
    }

    fn delete(&self, key: &str) -> Result<()> {
        let mut map = self.read_map()?;
        if map.remove(key).is_some() {
            self.write_map(&map)?;
        }
        Ok(())
    }
}

/// Redact a token for a log line: keep a short prefix, drop the rest.
pub fn redact_token(token: &str) -> String {
    let head: String = token.chars().take(4).collect();
    format!("{head}…<{} chars>", token.chars().count())
}

/// The service name every keyring entry is filed under.
pub const KEYRING_SERVICE: &str = "nidvue-drive";

/// Credentials in the OS keyring: Secret Service on Linux, the Security
/// framework on macOS, the Credential Manager on Windows.
///
/// Every operation maps the platform error onto [`NidvueError::Secret`], and a
/// missing entry is `Ok(None)` rather than an error, so the caller cannot tell a
/// fresh install from a keyring that lost its entry.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    /// A store filed under `service`.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// The default store, filed under [`KEYRING_SERVICE`].
    pub fn default_service() -> Self {
        Self::new(KEYRING_SERVICE)
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, key)
            .map_err(|e| NidvueError::Secret(format!("keyring entry {key}: {e}")))
    }
}

impl SecretStore for KeyringStore {
    fn load(&self, key: &str) -> Result<Option<SecretString>> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(SecretString::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(NidvueError::Secret(format!("keyring read {key}: {e}"))),
        }
    }

    fn store(&self, key: &str, value: &SecretString) -> Result<()> {
        self.entry(key)?
            .set_password(value.expose())
            .map_err(|e| NidvueError::Secret(format!("keyring write {key}: {e}")))
    }

    fn delete(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(NidvueError::Secret(format!("keyring delete {key}: {e}"))),
        }
    }
}

/// The store the client actually opened, and why.
#[derive(Clone)]
pub struct StoreSelection {
    /// The live store.
    pub store: Arc<dyn SecretStore>,
    /// Which backend it is.
    pub backend: SecretBackend,
    /// Set when the requested backend was not usable and the file store took
    /// over; carries the platform's own message.
    pub fallback_reason: Option<String>,
}

impl StoreSelection {
    /// One line for `doctor` and `status`.
    pub fn describe(&self) -> String {
        match &self.fallback_reason {
            None => self.backend.to_string(),
            Some(reason) => format!("{} (fell back from keyring: {reason})", self.backend),
        }
    }
}

/// Probe the OS keyring with a throwaway entry.
fn keyring_is_usable(store: &KeyringStore) -> Result<()> {
    let probe = "__nidvue_probe";
    let value = SecretString::new("probe");
    store.store(probe, &value)?;
    let read_back = store.load(probe)?;
    store.delete(probe)?;
    match read_back {
        Some(v) if v.expose() == "probe" => Ok(()),
        _ => Err(NidvueError::Secret(
            "keyring probe did not read back".into(),
        )),
    }
}

/// Open the credential store the configuration asks for.
///
/// `auto` prefers the OS keyring and records the reason when it is unavailable;
/// `keyring` is a hard requirement and fails instead of downgrading; `file` is
/// the encrypted file store and never probes the platform.
pub fn open_store(backend: SecretBackend, credentials_file: impl Into<PathBuf>) -> StoreSelection {
    let credentials_file = credentials_file.into();
    let file =
        || Arc::new(EncryptedFileStore::new(credentials_file.clone())) as Arc<dyn SecretStore>;
    let keyring_store = KeyringStore::default_service();
    match backend {
        SecretBackend::File => StoreSelection {
            store: file(),
            backend: SecretBackend::File,
            fallback_reason: None,
        },
        SecretBackend::Keyring => StoreSelection {
            store: Arc::new(KeyringStore::default_service()),
            backend: SecretBackend::Keyring,
            fallback_reason: None,
        },
        SecretBackend::Auto => match keyring_is_usable(&keyring_store) {
            Ok(()) => StoreSelection {
                store: Arc::new(keyring_store),
                backend: SecretBackend::Keyring,
                fallback_reason: None,
            },
            Err(e) => StoreSelection {
                store: file(),
                backend: SecretBackend::File,
                fallback_reason: Some(e.to_string()),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_encrypts() {
        let dir = tempfile::tempdir().unwrap();
        let store = EncryptedFileStore::new(dir.path().join("credentials.enc"));
        assert!(store.load("jotta").unwrap().is_none());
        store
            .store("jotta", &SecretString::new("super-secret-token"))
            .unwrap();
        let got = store.load("jotta").unwrap().unwrap();
        assert_eq!(got.expose(), "super-secret-token");

        // The plaintext must not be on disk.
        let raw = std::fs::read(dir.path().join("credentials.enc")).unwrap();
        assert!(!raw.windows(18).any(|w| w == b"super-secret-token"));

        // A second store with the same key file reads it back.
        let again = EncryptedFileStore::new(dir.path().join("credentials.enc"));
        assert_eq!(
            again.load("jotta").unwrap().unwrap().expose(),
            "super-secret-token"
        );

        store.delete("jotta").unwrap();
        assert!(store.load("jotta").unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = EncryptedFileStore::new(dir.path().join("credentials.enc"));
        store.store("k", &SecretString::new("v")).unwrap();
        for p in ["credentials.enc", "credentials.key"] {
            let mode = std::fs::metadata(dir.path().join(p))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "{p} mode {mode:o}");
        }
    }

    #[test]
    fn wrong_key_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.enc");
        let store = EncryptedFileStore::new(&path);
        store.store("k", &SecretString::new("v")).unwrap();
        // Replace the key with a different one.
        write_private(&dir.path().join("credentials.key"), &[7u8; 32]).unwrap();
        let err = store.load("k").unwrap_err();
        assert!(err.to_string().contains("decryption failed"));
    }

    #[test]
    fn secrets_do_not_print_themselves() {
        let s = SecretString::new("abcdef123456");
        assert_eq!(format!("{s:?}"), "<redacted 12 bytes>");
        assert_eq!(redact_token("abcdef123456"), "abcd…<12 chars>");
    }

    #[test]
    fn file_backend_is_selected_without_probing_the_platform() {
        let dir = tempfile::tempdir().unwrap();
        let sel = open_store(SecretBackend::File, dir.path().join("credentials.enc"));
        assert_eq!(sel.backend, SecretBackend::File);
        assert!(sel.fallback_reason.is_none());
        assert_eq!(sel.describe(), "file");
        sel.store.store("k", &SecretString::new("v")).unwrap();
        assert_eq!(sel.store.load("k").unwrap().unwrap().expose(), "v");
    }

    #[test]
    fn auto_always_lands_on_a_working_store() {
        let dir = tempfile::tempdir().unwrap();
        let sel = open_store(SecretBackend::Auto, dir.path().join("credentials.enc"));
        sel.store.store("k", &SecretString::new("v")).unwrap();
        assert_eq!(sel.store.load("k").unwrap().unwrap().expose(), "v");
        match sel.backend {
            SecretBackend::Keyring => assert!(sel.fallback_reason.is_none()),
            SecretBackend::File => {
                let reason = sel
                    .fallback_reason
                    .as_ref()
                    .expect("a fallback states its reason");
                assert!(!reason.is_empty());
                assert!(sel.describe().starts_with("file (fell back from keyring:"));
            }
            SecretBackend::Auto => panic!("auto resolves to a concrete backend"),
        }
    }

    #[test]
    fn backend_names_round_trip() {
        for name in ["auto", "file", "keyring"] {
            let parsed: SecretBackend = name.parse().unwrap();
            assert_eq!(parsed.to_string(), name);
        }
        assert!("nonsense".parse::<SecretBackend>().is_err());
    }

    #[test]
    #[ignore = "needs a running Secret Service on the session bus"]
    fn keyring_round_trips_through_the_os_store() {
        let store = KeyringStore::new("nidvue-drive-test");
        let key = "round-trip";
        store
            .store(key, &SecretString::new("keyring-value"))
            .unwrap();
        let got = store.load(key).unwrap().expect("stored value");
        assert_eq!(got.expose(), "keyring-value");
        store.delete(key).unwrap();
        assert!(store.load(key).unwrap().is_none());
        store.delete(key).unwrap();
    }
}
