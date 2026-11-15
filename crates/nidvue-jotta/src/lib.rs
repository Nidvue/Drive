// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Jottacloud API client.
//!
//! This is the only crate in the workspace that speaks HTTP. It owns:
//!
//! - authentication (Personal Login Token → OIDC token → refresh),
//! - the JFS XML codec and the `liststream` reader,
//! - the Allocate upload API (resume and MD5 de-duplication),
//! - rate limiting, backoff, single-flight coalescing and AIMD concurrency.
//!
//! The API surface is documented in `docs/jotta-api.md`.

pub mod otlp;

pub mod auth;
pub mod client;
pub mod ratelimit;
pub mod xml;

pub use auth::{AuthManager, LoginToken, TokenSet};
pub use client::{AllocateResponse, JottaClient, JottaConfig, Quota, RemoteFile, RemoteFolder};
pub use ratelimit::{Aimd, Coalescer, RetryPolicy, TokenBucket};
pub use xml::{parse_entry, parse_list_stream, ListStream, RemoteEntry, Stats};

/// Map an HTTP status and body to a typed error.
pub fn http_error(status: u16, body: &str) -> nidvue_core::NidvueError {
    use nidvue_core::NidvueError;
    match status {
        401 | 403 => NidvueError::Auth(format!("HTTP {status}: {}", first_line(body))),
        404 => NidvueError::NotFound(first_line(body)),
        409 => NidvueError::AlreadyExists(first_line(body)),
        429 => NidvueError::RateLimited {
            retry_after_ms: None,
        },
        408 | 500..=599 => NidvueError::Server {
            status,
            message: first_line(body),
        },
        _ => NidvueError::Server {
            status,
            message: first_line(body),
        },
    }
}

fn first_line(body: &str) -> String {
    let line = body.lines().next().unwrap_or("").trim();
    let line = line.replace(['\n', '\r'], " ");
    if line.len() > 300 {
        format!("{}…", &line[..300])
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nidvue_core::NidvueError;

    #[test]
    fn status_mapping() {
        assert!(matches!(http_error(401, "no"), NidvueError::Auth(_)));
        assert!(matches!(http_error(404, "gone"), NidvueError::NotFound(_)));
        assert!(matches!(
            http_error(409, "exists"),
            NidvueError::AlreadyExists(_)
        ));
        assert!(matches!(
            http_error(429, ""),
            NidvueError::RateLimited { .. }
        ));
        assert!(matches!(
            http_error(503, "busy"),
            NidvueError::Server { status: 503, .. }
        ));
    }

    #[test]
    fn bodies_are_truncated() {
        let e = http_error(500, &"x".repeat(1000));
        assert!(e.to_string().len() < 400);
    }
}
