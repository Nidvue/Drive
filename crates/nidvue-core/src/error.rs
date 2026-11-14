// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Structured errors. No stringly errors cross a crate or FFI boundary.

use std::path::Path;

/// Every failure in Nidvue Drive is one of these variants.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NidvueError {
    /// A local filesystem operation failed.
    #[error("io error at {path}: {source}")]
    Io {
        /// Path the operation was working on.
        path: String,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },
    /// A path was not a valid remote path.
    #[error("invalid path: {0}")]
    InvalidPath(String),
    /// Login or token refresh failed.
    #[error("authentication failed: {0}")]
    Auth(String),
    /// The remote or local entry does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The server rejected the request because it already exists.
    #[error("already exists: {0}")]
    AlreadyExists(String),
    /// Two divergent versions of one path were detected.
    #[error("conflict at {path}: {detail}")]
    Conflict {
        /// Conflicting path.
        path: String,
        /// Human readable explanation.
        detail: String,
    },
    /// The server asked us to slow down.
    #[error("rate limited (retry after {retry_after_ms:?} ms)")]
    RateLimited {
        /// Server-provided delay, if any.
        retry_after_ms: Option<u64>,
    },
    /// The server returned a non-success status we cannot classify further.
    #[error("server error {status}: {message}")]
    Server {
        /// HTTP status code.
        status: u16,
        /// Body or status text, redacted.
        message: String,
    },
    /// The transport failed.
    #[error("network error: {0}")]
    Network(String),
    /// The cache refused or failed an operation.
    #[error("cache error: {0}")]
    Cache(String),
    /// The metadata index or journal failed.
    #[error("database error: {0}")]
    Index(String),
    /// The journal was inconsistent with the on-disk state.
    #[error("journal error: {0}")]
    Journal(String),
    /// Configuration could not be read or was invalid.
    #[error("config error: {0}")]
    Config(String),
    /// The credential store failed.
    #[error("credential store error: {0}")]
    Secret(String),
    /// A plugin failed, trapped or violated its capabilities.
    #[error("plugin {plugin}: {message}")]
    Plugin {
        /// Plugin id.
        plugin: String,
        /// What went wrong.
        message: String,
    },
    /// Mounting or a kernel filesystem operation failed.
    #[error("mount error: {0}")]
    Mount(String),
    /// The operation was cancelled by the user or by shutdown.
    #[error("cancelled")]
    Cancelled,
    /// The platform does not support this operation.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// A hash or size check failed after a transfer.
    #[error("verification failed: {0}")]
    Verification(String),
    /// A bug: an invariant the code relies on did not hold.
    #[error("internal error: {0}")]
    Internal(String),
}

impl NidvueError {
    /// Attach a path to an I/O error.
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// True when retrying the identical request may succeed.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } => true,
            Self::Network(_) => true,
            Self::Server { status, .. } => *status == 408 || *status == 429 || *status >= 500,
            _ => false,
        }
    }

    /// True when the failure is a cancellation, which is not an error to report.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// The exit code `nidvue-cli` uses for this class of failure.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Auth(_) | Self::Secret(_) => 2,
            Self::NotFound(_) => 3,
            Self::Conflict { .. } => 4,
            Self::RateLimited { .. } => 5,
            Self::Cancelled => 130,
            _ => 1,
        }
    }
}

/// Result alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, NidvueError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_classification() {
        assert!(NidvueError::RateLimited { retry_after_ms: None }.is_retryable());
        assert!(NidvueError::Server { status: 503, message: String::new() }.is_retryable());
        assert!(!NidvueError::Server { status: 404, message: String::new() }.is_retryable());
        assert!(!NidvueError::Cancelled.is_retryable());
        assert_eq!(NidvueError::Cancelled.exit_code(), 130);
    }

    #[test]
    fn io_keeps_the_path() {
        let e = NidvueError::io("/tmp/x", std::io::Error::other("boom"));
        assert!(e.to_string().contains("/tmp/x"));
    }
}
