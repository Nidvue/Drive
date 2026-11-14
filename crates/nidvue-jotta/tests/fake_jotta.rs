// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Integration tests against a fake Jottacloud server.
//!
//! They cover the behaviour that is expensive to get wrong: the login exchange,
//! refresh-on-401, 429 backoff, partial listings, and the Allocate upload path
//! including resume and de-duplication.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use nidvue_core::path::RemotePath;
use nidvue_core::secret::{EncryptedFileStore, SecretStore};
use nidvue_jotta::auth::{AuthManager, TokenSet};
use nidvue_jotta::client::{JottaClient, JottaConfig};
use nidvue_jotta::xml::parse_list_stream;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN_JSON: &str = r#"{"access_token":"bearer-one","refresh_token":"refresh-one","expires_in":3600,"token_type":"Bearer"}"#;
const REFRESHED_JSON: &str = r#"{"access_token":"bearer-two","refresh_token":"refresh-two","expires_in":3600,"token_type":"Bearer"}"#;

fn login_token(well_known: &str) -> String {
    let json =
        format!(r#"{{"well_known_link":"{well_known}","username":"alice","auth_token":"pw"}}"#);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json)
}

struct Harness {
    server: MockServer,
    client: JottaClient,
    auth: Arc<AuthManager>,
    store: Arc<EncryptedFileStore>,
    dir: tempfile::TempDir,
}

async fn harness() -> Harness {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(EncryptedFileStore::new(dir.path().join("credentials.enc")));

    Mock::given(method("GET"))
        .and(path("/well-known"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_endpoint": format!("{}/token", server.uri())
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=password"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TOKEN_JSON))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(REFRESHED_JSON))
        .mount(&server)
        .await;

    let http = JottaClient::http_client("nidvue-test", Duration::from_secs(10)).unwrap();
    let auth = Arc::new(AuthManager::new(http.clone(), store.clone()));
    auth.login(&login_token(&format!("{}/well-known", server.uri())))
        .await
        .unwrap();

    let cfg = JottaConfig {
        base_url: format!("{}/jfs", server.uri()),
        allocate_url: format!("{}/files/v1", server.uri()),
        device: "Jotta".into(),
        mountpoint: "Archive".into(),
        username: "alice".into(),
        user_agent: "nidvue-test".into(),
        timeout: Duration::from_secs(10),
        requests_per_second: 1000.0,
        burst: 100,
        min_concurrency: 1,
        max_concurrency: 4,
    };
    let client = JottaClient::new(http, auth.clone(), cfg);
    Harness {
        server,
        client,
        auth,
        store,
        dir,
    }
}

#[tokio::test]
async fn login_exchanges_the_personal_token_and_persists_it() {
    let h = harness().await;
    assert_eq!(h.auth.username().await.as_deref(), Some("alice"));
    assert!(h.auth.is_logged_in().await);

    // The stored token can be restored.
    let raw = h.store.load("jotta.tokens").unwrap().unwrap();
    let set: TokenSet = serde_json::from_str(raw.expose()).unwrap();
    assert_eq!(set.username, "alice");
    assert_eq!(set.refresh_token.as_ref().unwrap().expose(), "refresh-one");

    // Nothing in the file is plaintext.
    let on_disk = std::fs::read(h.store_path()).unwrap();
    assert!(!on_disk.windows(10).any(|w| w == b"bearer-one"));
}

impl Harness {
    fn store_path(&self) -> std::path::PathBuf {
        self.dir.path().join("credentials.enc")
    }
}

#[tokio::test]
async fn refreshes_once_on_401_and_retries_the_request() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/x.txt"))
        .respond_with(ResponseTemplate::new(401).set_body_string("<error><code>401</code></error>"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/x.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<file name="x.txt"><currentRevision><state>COMPLETED</state><size>5</size>
               <md5>5D41402ABC4B2A76B9719D911017C592</md5></currentRevision></file>"#,
        ))
        .with_priority(2)
        .mount(&h.server)
        .await;

    let entry = h
        .client
        .stat(&RemotePath::parse("/x.txt").unwrap())
        .await
        .unwrap();
    assert_eq!(entry.size, 5);
    assert_eq!(
        entry.md5.as_deref(),
        Some("5d41402abc4b2a76b9719d911017c592")
    );
}

#[tokio::test]
async fn honours_retry_after_on_429() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/busy"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("<error><code>429</code></error>"),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/busy"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"<folder name="busy"><size>0</size></folder>"#),
        )
        .with_priority(2)
        .mount(&h.server)
        .await;

    let entry = h
        .client
        .stat(&RemotePath::parse("/busy").unwrap())
        .await
        .unwrap();
    assert_eq!(entry.name, "busy");
}

#[tokio::test]
async fn a_partial_listing_is_an_error_not_a_deletion_set() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/Docs"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<folder>
                 <file path="Docs" filename="a.txt" size="1"/>
                 <stats folders="0" files="2"/>
               </folder>"#,
        ))
        .mount(&h.server)
        .await;

    let err = h
        .client
        .list_stream(&RemotePath::parse("/Docs").unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("partial listing"), "{err}");
}

#[tokio::test]
async fn lists_with_liststream_mode() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/Docs"))
        .and(wiremock::matchers::query_param("mode", "liststream"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<folder>
                 <folder path="Docs/2026"/>
                 <file path="Docs" filename="a.txt" size="1" md5="0CC175B9C0F1B6A831C399E269772661"/>
                 <stats folders="1" files="1"/>
               </folder>"#,
        ))
        .mount(&h.server)
        .await;

    let listing = h
        .client
        .list_stream(&RemotePath::parse("/Docs").unwrap())
        .await
        .unwrap();
    assert_eq!(listing.files.len(), 1);
    assert_eq!(listing.folders.len(), 1);
    assert!(!listing.is_partial());
}

#[tokio::test]
async fn upload_resumes_from_the_allocate_position_and_verifies() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"hello world").unwrap();
    let md5 = nidvue_core::hash::md5_file(&file).unwrap();

    Mock::given(method("POST"))
        .and(path("/files/v1/allocate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "state": "INCOMPLETE",
            "upload_id": "u1",
            "upload_url": format!("{}/upload/u1", h.server.uri()),
            "bytes": 11,
            "resume_pos": 6,
            "path": "/jfs/Jotta/Archive/f.txt"
        })))
        .mount(&h.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/upload/u1"))
        .and(header("range", "bytes=6-10"))
        .and(header("content-length", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "path": "/jfs/Jotta/Archive/f.txt", "bytes": 11, "md5": md5
        })))
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/f.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<file name="f.txt"><currentRevision><state>COMPLETED</state>
               <size>11</size><md5>{md5}</md5></currentRevision></file>"#
        )))
        .mount(&h.server)
        .await;

    let entry = h
        .client
        .upload(&RemotePath::parse("/f.txt").unwrap(), &file, None)
        .await
        .unwrap();
    assert_eq!(entry.size, 11);
}

#[tokio::test]
async fn deduplicated_upload_sends_no_body() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dup.bin");
    std::fs::write(&file, b"same bytes").unwrap();
    let md5 = nidvue_core::hash::md5_file(&file).unwrap();

    Mock::given(method("POST"))
        .and(path("/files/v1/allocate"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "state": "COMPLETED", "bytes": 10, "resume_pos": 0
        })))
        .mount(&h.server)
        .await;
    // If the client sends the body anyway, this mock records the hit.
    Mock::given(method("POST"))
        .and(path("/upload/never"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/dup.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<file name="dup.bin"><currentRevision><state>COMPLETED</state>
               <size>10</size><md5>{md5}</md5></currentRevision></file>"#
        )))
        .mount(&h.server)
        .await;

    let entry = h
        .client
        .upload(&RemotePath::parse("/dup.bin").unwrap(), &file, None)
        .await
        .unwrap();
    assert_eq!(entry.size, 10);
}

#[tokio::test]
async fn a_size_mismatch_after_upload_is_a_failure() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.bin");
    std::fs::write(&file, b"12345").unwrap();

    Mock::given(method("POST"))
        .and(path("/files/v1/allocate"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "state": "COMPLETED", "bytes": 5, "resume_pos": 0
        })))
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jfs/alice/Jotta/Archive/bad.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<file name="bad.bin"><currentRevision><state>COMPLETED</state>
               <size>4</size><md5>e10adc3949ba59abbe56e057f20f883e</md5></currentRevision></file>"#,
        ))
        .mount(&h.server)
        .await;

    let err = h
        .client
        .upload(&RemotePath::parse("/bad.bin").unwrap(), &file, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("verification failed"), "{err}");
}

#[tokio::test]
async fn parses_a_streamed_listing_from_a_reader() {
    // The reader path is what `list_stream` uses; this pins it independently.
    let xml = r#"<folder><file path="A" filename="one" size="1"/><stats folders="0" files="1"/></folder>"#;
    let parsed = parse_list_stream(xml).unwrap();
    assert_eq!(parsed.files.len(), 1);
}
