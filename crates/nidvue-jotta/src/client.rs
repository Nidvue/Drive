// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The Jottacloud client: JFS reads and writes, Allocate uploads, and the
//! policy that keeps the network healthy.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{StreamExt, TryStreamExt};
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::hash::{digests_match, md5_file};
use nidvue_core::path::RemotePath;
use nidvue_core::time::now_ms;
use nidvue_core::types::EntryKind;
use reqwest::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::{ReaderStream, StreamReader, SyncIoBridge};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::auth::AuthManager;
use crate::ratelimit::{Aimd, Coalescer, RetryPolicy, TokenBucket};
use crate::xml::{self, ListStream, RemoteEntry};

pub use crate::xml::{RemoteFile, RemoteFolder, Stats};

/// Everything the client needs to know about where it is talking.
#[derive(Debug, Clone)]
pub struct JottaConfig {
    /// JFS base URL, without a trailing slash.
    pub base_url: String,
    /// Allocate API base URL, without a trailing slash.
    pub allocate_url: String,
    /// Jottacloud device.
    pub device: String,
    /// Jottacloud mountpoint.
    pub mountpoint: String,
    /// The account's internal username.
    pub username: String,
    /// User agent.
    pub user_agent: String,
    /// Per-request timeout.
    pub timeout: Duration,
    /// Token bucket rate.
    pub requests_per_second: f64,
    /// Token bucket burst.
    pub burst: u32,
    /// AIMD floor.
    pub min_concurrency: u32,
    /// AIMD ceiling.
    pub max_concurrency: u32,
}

impl JottaConfig {
    /// Build from the application configuration.
    pub fn from_app(cfg: &nidvue_core::config::AppConfig, username: impl Into<String>) -> Self {
        Self {
            base_url: cfg.network.base_url.trim_end_matches('/').to_string(),
            allocate_url: cfg.network.allocate_url.trim_end_matches('/').to_string(),
            device: cfg.network.device.clone(),
            mountpoint: cfg.network.mountpoint.clone(),
            username: username.into(),
            user_agent: cfg.network.user_agent.clone(),
            timeout: Duration::from_millis(cfg.network.timeout_ms),
            requests_per_second: cfg.network.requests_per_second,
            burst: cfg.network.burst,
            min_concurrency: cfg.network.min_concurrency,
            max_concurrency: cfg.network.max_concurrency,
        }
    }

    /// The JFS URL for a remote path.
    pub fn jfs_url(&self, path: &RemotePath) -> String {
        let suffix = path.as_str().trim_start_matches('/');
        if suffix.is_empty() {
            format!(
                "{}/{}/{}/{}",
                self.base_url, self.username, self.device, self.mountpoint
            )
        } else {
            format!(
                "{}/{}/{}/{}{}",
                self.base_url,
                self.username,
                self.device,
                self.mountpoint,
                path.encoded()
            )
        }
    }

    /// The absolute path the Allocate API expects in its JSON body.
    pub fn allocate_path(&self, path: &RemotePath) -> String {
        format!("/jfs/{}/{}{}", self.device, self.mountpoint, path.encoded())
    }
}

/// The response to an Allocate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocateResponse {
    /// File name.
    #[serde(default)]
    pub name: String,
    /// Path.
    #[serde(default)]
    pub path: String,
    /// `COMPLETED` means the content is already there (de-duplicated).
    #[serde(default)]
    pub state: String,
    /// Upload id.
    #[serde(default)]
    pub upload_id: String,
    /// Pre-signed upload URL.
    #[serde(default)]
    pub upload_url: String,
    /// Total bytes.
    #[serde(default)]
    pub bytes: u64,
    /// How many bytes the server already has.
    #[serde(default)]
    pub resume_pos: u64,
}

/// Account quota, from the customer endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Quota {
    /// Bytes used.
    #[serde(default)]
    pub usage: u64,
    /// Bytes allowed.
    #[serde(default)]
    pub quota: u64,
    /// Account is read-locked.
    #[serde(default)]
    pub read_locked: bool,
    /// Account is write-locked.
    #[serde(default)]
    pub write_locked: bool,
}

/// Called with `(done, total)` as a transfer progresses.
pub type ProgressFn = Arc<dyn Fn(u64, u64) + Send + Sync>;

#[derive(Debug, Clone)]
enum Body {
    None,
    Json(serde_json::Value),
    File {
        path: PathBuf,
        offset: u64,
        len: u64,
    },
}

/// The Jottacloud client.
pub struct JottaClient {
    http: reqwest::Client,
    auth: Arc<AuthManager>,
    cfg: JottaConfig,
    bucket: TokenBucket,
    aimd: Aimd,
    retry: RetryPolicy,
    stat_cache: Coalescer<String, RemoteEntry>,
    cancel: CancellationToken,
}

impl JottaClient {
    /// Build a client. `auth` must already be logged in or restorable.
    pub fn new(http: reqwest::Client, auth: Arc<AuthManager>, cfg: JottaConfig) -> Self {
        Self {
            http,
            auth,
            bucket: TokenBucket::new(cfg.requests_per_second, cfg.burst),
            aimd: Aimd::new(cfg.min_concurrency, cfg.max_concurrency),
            retry: RetryPolicy::default(),
            stat_cache: Coalescer::new(Duration::from_millis(250)),
            cfg,
            cancel: CancellationToken::new(),
        }
    }

    /// Build a default HTTP client with the configured timeout and user agent.
    pub fn http_client(user_agent: &str, timeout: Duration) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| NidvueError::Network(format!("http client: {e}")))
    }

    /// The configuration in use.
    pub fn config(&self) -> &JottaConfig {
        &self.cfg
    }

    /// The current adaptive concurrency.
    pub fn concurrency(&self) -> u32 {
        self.aimd.current()
    }

    /// A token that cancels every in-flight request when cancelled.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// List a directory tree with `mode=liststream`.
    pub async fn list_stream(&self, path: &RemotePath) -> Result<ListStream> {
        let url = format!("{}?mode=liststream", self.cfg.jfs_url(path));
        let response = self.send(Method::GET, &url, Body::None).await?;
        let stream = response.bytes_stream().map_err(std::io::Error::other);
        let async_read = StreamReader::new(stream);
        let parsed = tokio::task::spawn_blocking(move || {
            let bridge = SyncIoBridge::new(async_read);
            xml::parse_list_stream_reader(bridge)
        })
        .await
        .map_err(|e| NidvueError::Internal(format!("listing task: {e}")))??;
        if parsed.is_partial() {
            // A partial listing must never be read as a set of deletions.
            return Err(NidvueError::Server {
                status: 0,
                message: format!(
                    "partial listing for {path}: stats {:?} vs {} files, {} folders",
                    parsed.stats,
                    parsed.files.len(),
                    parsed.folders.len()
                ),
            });
        }
        Ok(parsed)
    }

    /// Stat one path. Concurrent calls for the same path share one request.
    pub async fn stat(&self, path: &RemotePath) -> Result<RemoteEntry> {
        let key = path.as_str().to_string();
        self.stat_cache
            .run(key, || async {
                let url = self.cfg.jfs_url(path);
                let response = self.send(Method::GET, &url, Body::None).await?;
                let body = response
                    .text()
                    .await
                    .map_err(|e| NidvueError::Network(e.to_string()))?;
                xml::parse_entry(&body)
            })
            .await
    }

    /// Create a directory (idempotent: an existing directory is success).
    pub async fn mkdir(&self, path: &RemotePath) -> Result<()> {
        if path.is_root() {
            return Ok(());
        }
        let url = format!("{}?mkDir=true", self.cfg.jfs_url(path));
        match self.send(Method::POST, &url, Body::None).await {
            Ok(_) => Ok(()),
            Err(NidvueError::AlreadyExists(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Delete a path. Without `permanent` it goes to the Jottacloud trash.
    pub async fn delete(&self, path: &RemotePath, permanent: bool) -> Result<()> {
        let flag = if permanent { "rm" } else { "dl" };
        let url = format!("{}?{flag}=true", self.cfg.jfs_url(path));
        match self.send(Method::POST, &url, Body::None).await {
            Ok(_) => Ok(()),
            Err(NidvueError::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Move or rename one path. One request, not a copy and delete.
    pub async fn rename(&self, from: &RemotePath, to: &RemotePath) -> Result<()> {
        if from == to {
            return Ok(());
        }
        if let Some(parent) = to.parent() {
            if !parent.is_root() {
                self.mkdir(&parent).await?;
            }
        }
        let url = format!(
            "{}?mv={}",
            self.cfg.jfs_url(from),
            nidvue_core::path::encode_segment(&self.cfg.allocate_path(to))
        );
        self.send(Method::POST, &url, Body::None).await.map(|_| ())
    }

    /// Account quota and lock state.
    pub async fn quota(&self) -> Result<Quota> {
        let url = "https://api.jottacloud.com/account/v1/customer";
        let response = self.send(Method::GET, url, Body::None).await?;
        response
            .json()
            .await
            .map_err(|e| NidvueError::Network(format!("quota body: {e}")))
    }

    /// Download a file into an async writer, reporting progress.
    ///
    /// The caller is responsible for writing to a temp file and renaming; this
    /// function never touches the final destination.
    pub async fn download_to<W>(
        &self,
        path: &RemotePath,
        writer: &mut W,
        expected_size: Option<u64>,
        progress: Option<ProgressFn>,
    ) -> Result<u64>
    where
        W: tokio::io::AsyncWrite + Unpin + Send + ?Sized,
    {
        let url = self.cfg.jfs_url(path);
        let response = self.send(Method::GET, &url, Body::None).await?;
        let total = response.content_length().or(expected_size).unwrap_or(0);
        let mut stream = response.bytes_stream();
        let mut done = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| NidvueError::Network(format!("download {path}: {e}")))?;
            tokio::io::AsyncWriteExt::write_all(writer, &chunk)
                .await
                .map_err(|e| NidvueError::Io {
                    path: path.to_string(),
                    source: e,
                })?;
            done += chunk.len() as u64;
            if let Some(cb) = &progress {
                cb(done, total);
            }
        }
        tokio::io::AsyncWriteExt::flush(writer)
            .await
            .map_err(|e| NidvueError::Io {
                path: path.to_string(),
                source: e,
            })?;
        if let Some(expected) = expected_size {
            if done != expected {
                return Err(NidvueError::Verification(format!(
                    "{path}: downloaded {done} bytes, expected {expected}"
                )));
            }
        }
        Ok(done)
    }

    /// Upload a local file through the Allocate API.
    ///
    /// The MD5 is computed first (the API requires it), the file is allocated
    /// (which may de-duplicate or resume), the remaining bytes are sent, and
    /// the committed object is re-read to verify size and MD5.
    pub async fn upload(
        &self,
        remote: &RemotePath,
        local: &Path,
        progress: Option<ProgressFn>,
    ) -> Result<RemoteEntry> {
        let local_owned = local.to_path_buf();
        let (md5, size) = tokio::task::spawn_blocking(move || {
            let md5 = md5_file(&local_owned)?;
            let size = std::fs::metadata(&local_owned)
                .map_err(|e| NidvueError::io(&local_owned, e))?
                .len();
            Ok::<_, NidvueError>((md5, size))
        })
        .await
        .map_err(|e| NidvueError::Internal(format!("hash task: {e}")))??;

        let modified = std::fs::metadata(local)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| nidvue_core::time::iso8601(d.as_millis() as i64))
            .unwrap_or_else(|| nidvue_core::time::iso8601(now_ms()));

        let allocate = serde_json::json!({
            "bytes": size,
            "md5": md5,
            "created": modified,
            "modified": modified,
            "path": self.cfg.allocate_path(remote),
        });
        let url = format!("{}/allocate", self.cfg.allocate_url);
        let response = self.send(Method::POST, &url, Body::Json(allocate)).await?;
        let allocated: AllocateResponse = response
            .json()
            .await
            .map_err(|e| NidvueError::Network(format!("allocate body: {e}")))?;

        if !allocated.state.eq_ignore_ascii_case("COMPLETED") {
            let remaining = size.saturating_sub(allocated.resume_pos);
            debug!(
                path = %remote,
                size,
                resume = allocated.resume_pos,
                remaining,
                "uploading allocated file"
            );
            if allocated.upload_url.is_empty() {
                return Err(NidvueError::Server {
                    status: 0,
                    message: "allocate returned no upload_url".into(),
                });
            }
            let body = Body::File {
                path: local.to_path_buf(),
                offset: allocated.resume_pos,
                len: remaining,
            };
            let mut request = self
                .http
                .request(Method::POST, &allocated.upload_url)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .header(reqwest::header::CONTENT_LENGTH, remaining);
            if allocated.resume_pos != 0 {
                request = request.header(
                    reqwest::header::RANGE,
                    format!("bytes={}-{}", allocated.resume_pos, size.saturating_sub(1)),
                );
            }
            let request = self.with_body(request, body).await?;
            let response = self
                .execute(request, Method::POST, &allocated.upload_url)
                .await?;
            if let Some(cb) = &progress {
                cb(size, size);
            }
            let _ = response.bytes().await;
        }

        // Verify what the server now holds. A size or MD5 mismatch is a failed
        // transfer, never a success with a warning.
        let entry = self.stat(remote).await?;
        if entry.size != size {
            return Err(NidvueError::Verification(format!(
                "{remote}: server has {} bytes, uploaded {size}",
                entry.size
            )));
        }
        if let Some(server_md5) = &entry.md5 {
            if !digests_match(server_md5, &md5) {
                return Err(NidvueError::Verification(format!(
                    "{remote}: server md5 {server_md5} != local {md5}"
                )));
            }
        }
        Ok(entry)
    }

    /// Check that the remote entry matches an expected size and MD5.
    pub async fn verify(&self, remote: &RemotePath, size: u64, md5: &str) -> Result<bool> {
        let entry = self.stat(remote).await?;
        Ok(entry.size == size
            && entry
                .md5
                .as_deref()
                .map(|m| digests_match(m, md5))
                .unwrap_or(true))
    }

    /// Whether a path exists (404 is `false`, not an error).
    pub async fn exists(&self, path: &RemotePath) -> Result<bool> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(NidvueError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The kind of an existing path, if any.
    pub async fn kind_of(&self, path: &RemotePath) -> Result<Option<EntryKind>> {
        match self.stat(path).await {
            Ok(e) => Ok(Some(e.kind)),
            Err(NidvueError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn with_body(
        &self,
        request: reqwest::RequestBuilder,
        body: Body,
    ) -> Result<reqwest::RequestBuilder> {
        Ok(match body {
            Body::None => request,
            Body::Json(value) => request.json(&value),
            Body::File { path, offset, len } => {
                let mut file = tokio::fs::File::open(&path)
                    .await
                    .map_err(|e| NidvueError::io(&path, e))?;
                if offset > 0 {
                    file.seek(std::io::SeekFrom::Start(offset))
                        .await
                        .map_err(|e| NidvueError::io(&path, e))?;
                }
                let reader = tokio::io::BufReader::new(file.take(len));
                request.body(reqwest::Body::wrap_stream(ReaderStream::new(reader)))
            }
        })
    }

    /// Send a request with authentication, rate limiting, retry and refresh.
    async fn send(&self, method: Method, url: &str, body: Body) -> Result<Response> {
        let request = self.http.request(method.clone(), url);
        let request = self.with_body(request, body.clone()).await?;
        self.execute(request, method, url).await
    }

    async fn execute(
        &self,
        request: reqwest::RequestBuilder,
        _method: Method,
        url: &str,
    ) -> Result<Response> {
        let mut attempt: u32 = 1;
        let mut refreshed = false;
        // A streaming body (an upload) cannot be cloned, so it gets exactly one
        // attempt. The upload state machine resumes it from `resume_pos` on the
        // next pass, which is why that is not a silent data loss.
        let mut pending = Some(request);
        loop {
            self.bucket.acquire().await;
            let token = self.auth.access_token().await?;
            let rb = match pending.as_ref().and_then(|r| r.try_clone()) {
                Some(clone) => clone,
                None => pending.take().ok_or_else(|| {
                    NidvueError::Internal("request body cannot be retried".into())
                })?,
            };
            let rb = rb.bearer_auth(token.expose());
            let outcome = tokio::select! {
                _ = self.cancel.cancelled() => return Err(NidvueError::Cancelled),
                r = rb.send() => r,
            };
            let response = match outcome {
                Ok(r) => r,
                Err(e) => {
                    if self.retry.should_retry(attempt) {
                        let delay = self.retry.delay(attempt, None);
                        warn!(url, attempt, ?delay, error = %e, "request failed, retrying");
                        attempt += 1;
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(NidvueError::Network(format!("{url}: {e}")));
                }
            };
            let status = response.status();
            if status == StatusCode::UNAUTHORIZED && !refreshed {
                refreshed = true;
                attempt += 1;
                debug!(url, "401, refreshing token");
                self.auth.refresh_force().await?;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                self.aimd.on_congestion();
                if self.retry.should_retry(attempt) {
                    let hint = retry_after(&response);
                    let delay = self.retry.delay(attempt, hint);
                    warn!(url, attempt, %status, ?delay, "server congestion, backing off");
                    attempt += 1;
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
            self.aimd.on_success();
            if status.is_success() {
                return Ok(response);
            }
            let body = response.text().await.unwrap_or_default();
            return Err(crate::http_error(status.as_u16(), &body));
        }
    }
}

fn retry_after(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_built_from_the_configuration() {
        let cfg = JottaConfig {
            base_url: "https://jfs.example/jfs".into(),
            allocate_url: "https://api.example/files/v1".into(),
            device: "Jotta".into(),
            mountpoint: "Archive".into(),
            username: "alice".into(),
            user_agent: "test".into(),
            timeout: Duration::from_secs(5),
            requests_per_second: 10.0,
            burst: 10,
            min_concurrency: 1,
            max_concurrency: 4,
        };
        assert_eq!(
            cfg.jfs_url(&RemotePath::parse("/Docs/a b.txt").unwrap()),
            "https://jfs.example/jfs/alice/Jotta/Archive/Docs/a%20b.txt"
        );
        assert_eq!(
            cfg.jfs_url(&RemotePath::root()),
            "https://jfs.example/jfs/alice/Jotta/Archive"
        );
        assert_eq!(
            cfg.allocate_path(&RemotePath::parse("/Docs/a b.txt").unwrap()),
            "/jfs/Jotta/Archive/Docs/a%20b.txt"
        );
    }
}
