// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Content hashing.
//!
//! Nidvue uses SHA-256 as its local content identity (the blob store key) and
//! MD5 only where the Jottacloud Allocate API demands it for de-duplication and
//! post-upload verification.

use std::fmt;
use std::io::{BufReader, Read};
use std::path::Path;

use md5::Md5;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{NidvueError, Result};

/// A lower-case hex SHA-256 digest, 64 characters.
#[derive(
    Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct Hash(String);

impl Hash {
    /// Wrap an already-computed lower-case hex digest.
    pub fn from_hex(hex: impl Into<String>) -> Result<Self> {
        let hex = hex.into().to_ascii_lowercase();
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(NidvueError::Internal(format!("not a sha256 digest: {hex}")));
        }
        Ok(Self(hex))
    }

    /// Hash a byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hex::encode(hasher.finalize()))
    }

    /// The hex digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// First two hex characters, used as the shard directory.
    pub fn shard(&self) -> &str {
        &self.0[..2]
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({}…)", &self.0[..8])
    }
}

/// A SHA-256 plus the byte count it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashAndSize {
    /// Content hash.
    pub hash: Hash,
    /// Number of bytes read.
    pub size: u64,
}

/// Hash a file, returning the digest and the number of bytes read.
pub fn hash_file(path: &Path) -> Result<HashAndSize> {
    let file = std::fs::File::open(path).map_err(|e| NidvueError::io(path, e))?;
    hash_reader(BufReader::new(file))
}

/// Hash any reader, returning the digest and the number of bytes read.
pub fn hash_reader(mut reader: impl Read) -> Result<HashAndSize> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut size = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| NidvueError::Io {
            path: "<stream>".into(),
            source: e,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok(HashAndSize {
        hash: Hash(hex::encode(hasher.finalize())),
        size,
    })
}

/// MD5 of a file, as lower-case hex, for the Jottacloud Allocate API.
pub fn md5_file(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|e| NidvueError::io(path, e))?;
    md5_reader(BufReader::new(file))
}

/// MD5 of a reader, as lower-case hex.
pub fn md5_reader(mut reader: impl Read) -> Result<String> {
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| NidvueError::Io {
            path: "<stream>".into(),
            source: e,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Constant-shape comparison of two digests, used for verification.
pub fn digests_match(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        // SHA-256("abc")
        assert_eq!(
            Hash::of(b"abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(Hash::of(b"").shard(), "e3");
        assert_eq!(
            md5_reader(&b"abc"[..]).unwrap(),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn hex_validation() {
        assert!(Hash::from_hex("A".repeat(64)).is_ok());
        assert!(Hash::from_hex("zz".repeat(32)).is_err());
        assert!(Hash::from_hex("abc").is_err());
    }

    #[test]
    fn file_hash_matches_reader_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        std::fs::write(&path, b"hello nidvue").unwrap();
        let a = hash_file(&path).unwrap();
        let b = hash_reader(&b"hello nidvue"[..]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.size, 12);
        assert!(digests_match(
            &md5_file(&path).unwrap(),
            &md5_reader(&b"hello nidvue"[..]).unwrap()
        ));
    }
}
