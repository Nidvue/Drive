// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Jottacloud authentication.
//!
//! A Personal Login Token is a base64url-encoded JSON object containing a
//! well-known link, a username and a single-use auth token. It is exchanged for
//! an OIDC bearer token, which is refreshed before expiry and once on a 401.
//!
//! See `docs/jotta-api.md` §1.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use nidvue_core::error::{NidvueError, Result};
use nidvue_core::secret::{SecretStore, SecretString};
use nidvue_core::time::now_ms;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const CLIENT_ID: &str = "jottacli";
const STORE_KEY: &str = "jotta.tokens";
/// Refresh this many milliseconds before the access token expires.
const REFRESH_MARGIN_MS: i64 = 60_000;

/// A decoded Personal Login Token.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginToken {
    /// OIDC discovery document URL.
    pub well_known_link: String,
    /// Jottacloud username.
    pub username: String,
    /// The single-use auth token, treated as a password.
    pub auth_token: SecretString,
}

impl LoginToken {
    /// Decode the base64url JSON a user pastes from the Jottacloud web UI.
    pub fn decode(raw: &str) -> Result<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(NidvueError::Auth("empty login token".into()));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(raw.trim_end_matches('='))
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(raw))
            .map_err(|_| NidvueError::Auth("login token is not base64".into()))?;
        #[derive(Deserialize)]
        struct Raw {
            well_known_link: String,
            username: String,
            auth_token: String,
        }
        let parsed: Raw = serde_json::from_slice(&bytes)
            .map_err(|e| NidvueError::Auth(format!("login token is not the expected JSON: {e}")))?;
        if parsed.well_known_link.is_empty() || parsed.username.is_empty() {
            return Err(NidvueError::Auth("login token is missing fields".into()));
        }
        Ok(Self {
            well_known_link: parsed.well_known_link,
            username: parsed.username,
            auth_token: SecretString::new(parsed.auth_token),
        })
    }
}

/// A bearer token and everything needed to refresh it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// The bearer token.
    pub access_token: SecretString,
    /// The refresh token, rotated on every refresh.
    pub refresh_token: Option<SecretString>,
    /// Expiry, ms since the epoch.
    pub expires_at_ms: i64,
    /// The token endpoint the token came from.
    pub token_endpoint: String,
    /// The account username.
    pub username: String,
}

impl TokenSet {
    /// True when the token is expired or about to expire.
    pub fn is_expiring(&self) -> bool {
        now_ms() + REFRESH_MARGIN_MS >= self.expires_at_ms
    }
}

#[derive(Deserialize)]
struct WellKnown {
    token_endpoint: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: i64,
}

/// Owns the token lifecycle: login, restore, refresh, and persistence.
pub struct AuthManager {
    http: reqwest::Client,
    store: Arc<dyn SecretStore>,
    token: Mutex<Option<TokenSet>>,
    /// Serialises refreshes so a burst of 401s causes exactly one refresh.
    refresh_lock: Mutex<()>,
}

impl AuthManager {
    /// Build an auth manager over an HTTP client and a credential store.
    pub fn new(http: reqwest::Client, store: Arc<dyn SecretStore>) -> Self {
        Self {
            http,
            store,
            token: Mutex::new(None),
            refresh_lock: Mutex::new(()),
        }
    }

    /// Exchange a Personal Login Token and persist the result.
    pub async fn login(&self, login_token: &str) -> Result<String> {
        let token = LoginToken::decode(login_token)?;
        self.exchange_password(
            &token.well_known_link,
            &token.username,
            token.auth_token.clone(),
        )
        .await
    }

    /// Log in with the account password instead of a Personal Login Token.
    ///
    /// The flow is the same OIDC password grant the token exchange uses; the
    /// difference is where the credentials come from. The password is held in a
    /// [`SecretString`], used for this one request and dropped: it is never
    /// stored, never written to the config and never formatted into a log line.
    /// `well_known` is the account's OIDC discovery URL — a Personal Login Token
    /// carries it, which is why a token login does not need it.
    pub async fn login_with_password(
        &self,
        well_known: &str,
        username: &str,
        password: SecretString,
    ) -> Result<String> {
        self.exchange_password(well_known, username, password).await
    }

    async fn exchange_password(
        &self,
        well_known: &str,
        username: &str,
        password: SecretString,
    ) -> Result<String> {
        let discovery: WellKnown = self
            .http
            .get(well_known)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| NidvueError::Network(format!("OIDC discovery: {e}")))?
            .error_for_status()
            .map_err(|e| NidvueError::Auth(format!("OIDC discovery: {e}")))?
            .json()
            .await
            .map_err(|e| NidvueError::Auth(format!("OIDC discovery body: {e}")))?;

        let form = [
            ("client_id", CLIENT_ID),
            ("grant_type", "password"),
            ("username", username),
            ("password", password.expose()),
            ("scope", "openid offline_access"),
        ];
        let response = self
            .http
            .post(&discovery.token_endpoint)
            .form(&form)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| NidvueError::Network(format!("token exchange: {e}")))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(NidvueError::Auth(format!(
                "token exchange failed with {status}: {}",
                redact_body(&body)
            )));
        }
        let parsed: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| NidvueError::Auth(format!("token response: {e}")))?;
        let set = TokenSet {
            access_token: SecretString::new(parsed.access_token),
            refresh_token: parsed.refresh_token.map(SecretString::new),
            expires_at_ms: now_ms() + parsed.expires_in.max(60) * 1000,
            token_endpoint: discovery.token_endpoint,
            username: username.to_string(),
        };
        self.persist(&set)?;
        *self.token.lock().await = Some(set);
        Ok(username.to_string())
    }

    /// Load a persisted token. Returns false when none is stored.
    pub async fn restore(&self) -> Result<bool> {
        // A store that cannot answer is "no token yet", not "cannot start":
        // the platform keyring can be present and still fail a read (a locked
        // collection, a session bus that changed under us). Failing open would
        // make the whole client unusable for a problem the user fixes by
        // logging in again.
        let raw = match self.store.load(STORE_KEY) {
            Ok(Some(raw)) => raw,
            Ok(None) => return Ok(false),
            Err(e) => {
                tracing::warn!(error = %e, "stored token unreadable; continuing without it");
                return Ok(false);
            }
        };
        let set: TokenSet = serde_json::from_str(raw.expose())
            .map_err(|e| NidvueError::Secret(format!("stored token is corrupt: {e}")))?;
        *self.token.lock().await = Some(set);
        Ok(true)
    }

    /// The account username, when logged in.
    pub async fn username(&self) -> Option<String> {
        self.token.lock().await.as_ref().map(|t| t.username.clone())
    }

    /// True when a token is present (it may still need a refresh).
    pub async fn is_logged_in(&self) -> bool {
        self.token.lock().await.is_some()
    }

    /// Forget the token, in memory and on disk.
    pub async fn logout(&self) -> Result<()> {
        *self.token.lock().await = None;
        self.store.delete(STORE_KEY)
    }

    /// A usable bearer token, refreshing it when it is close to expiry.
    pub async fn access_token(&self) -> Result<SecretString> {
        let needs_refresh = {
            let guard = self.token.lock().await;
            match guard.as_ref() {
                Some(t) => t.is_expiring(),
                None => return Err(NidvueError::Auth("not logged in".into())),
            }
        };
        if needs_refresh {
            self.refresh().await?;
        }
        let guard = self.token.lock().await;
        guard
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| NidvueError::Auth("not logged in".into()))
    }

    /// Install a token set directly (used by `login` and by tests).
    pub async fn install_token(&self, set: TokenSet) -> Result<()> {
        self.persist(&set)?;
        *self.token.lock().await = Some(set);
        Ok(())
    }

    /// Refresh if the token is close to expiry.
    pub async fn refresh(&self) -> Result<()> {
        let expiring = {
            let guard = self.token.lock().await;
            match guard.as_ref() {
                Some(t) => t.is_expiring(),
                None => return Err(NidvueError::Auth("not logged in".into())),
            }
        };
        if !expiring {
            return Ok(());
        }
        self.refresh_force().await
    }

    /// Refresh unconditionally. A 401 means the token is invalid whatever its
    /// expiry says, so the client calls this on an authentication failure.
    pub async fn refresh_force(&self) -> Result<()> {
        let _guard = self.refresh_lock.lock().await;
        // Another caller may have refreshed while we waited: use the new token.
        {
            let guard = self.token.lock().await;
            if let Some(t) = guard.as_ref() {
                if !t.is_expiring() && t.expires_at_ms > now_ms() + REFRESH_MARGIN_MS {
                    return Ok(());
                }
            }
        }
        let current = {
            let guard = self.token.lock().await;
            guard
                .clone()
                .ok_or_else(|| NidvueError::Auth("not logged in".into()))?
        };
        let refresh_token = current
            .refresh_token
            .as_ref()
            .ok_or_else(|| NidvueError::Auth("no refresh token; log in again".into()))?;

        let form = [
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose()),
        ];
        let response = self
            .http
            .post(&current.token_endpoint)
            .form(&form)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| NidvueError::Network(format!("token refresh: {e}")))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(NidvueError::Auth(format!(
                "token refresh failed with {status}: {}",
                redact_body(&body)
            )));
        }
        let parsed: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| NidvueError::Auth(format!("refresh response: {e}")))?;
        let set = TokenSet {
            access_token: SecretString::new(parsed.access_token),
            // Rotation: a new refresh token replaces the old one.
            refresh_token: parsed
                .refresh_token
                .map(SecretString::new)
                .or(current.refresh_token.clone()),
            expires_at_ms: now_ms() + parsed.expires_in.max(60) * 1000,
            token_endpoint: current.token_endpoint,
            username: current.username,
        };
        self.persist(&set)?;
        *self.token.lock().await = Some(set);
        Ok(())
    }

    fn persist(&self, set: &TokenSet) -> Result<()> {
        let json = serde_json::to_string(set)
            .map_err(|e| NidvueError::Secret(format!("serialise token: {e}")))?;
        self.store.store(STORE_KEY, &SecretString::new(json))
    }
}

/// Keep server error bodies short and never echo a token-shaped value.
fn redact_body(body: &str) -> String {
    let mut out = body.replace(['\n', '\r'], " ");
    for marker in ["access_token", "refresh_token", "id_token"] {
        if let Some(idx) = out.find(marker) {
            out.truncate(idx);
            out.push_str("…(redacted)");
            break;
        }
    }
    if out.len() > 200 {
        out.truncate(200);
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nidvue_core::secret::EncryptedFileStore;

    fn login_token_json(well_known: &str) -> String {
        let raw = format!(
            r#"{{"well_known_link":"{well_known}","username":"alice","auth_token":"pw-123"}}"#
        );
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
    }

    struct BrokenStore;

    impl SecretStore for BrokenStore {
        fn load(&self, _key: &str) -> Result<Option<SecretString>> {
            Err(nidvue_core::NidvueError::Secret(
                "dbus error: client public key size is invalid".into(),
            ))
        }
        fn store(&self, _key: &str, _value: &SecretString) -> Result<()> {
            Ok(())
        }
        fn delete(&self, _key: &str) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn an_unreadable_store_does_not_stop_the_client() {
        let auth = AuthManager::new(reqwest::Client::new(), Arc::new(BrokenStore));
        assert!(
            !auth.restore().await.unwrap(),
            "a store that cannot answer means no token, not a failure"
        );
        assert!(!auth.is_logged_in().await);
    }

    #[test]
    fn decodes_a_login_token() {
        let token = LoginToken::decode(&login_token_json("https://id.example/wk")).unwrap();
        assert_eq!(token.username, "alice");
        assert_eq!(token.auth_token.expose(), "pw-123");
        assert!(!format!("{token:?}").contains("pw-123"));
    }

    #[test]
    fn rejects_bad_login_tokens() {
        assert!(LoginToken::decode("").is_err());
        assert!(LoginToken::decode("not base64!!").is_err());
        let not_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("hello");
        assert!(LoginToken::decode(&not_json).is_err());
    }

    #[test]
    fn token_expiry_uses_a_margin() {
        let set = TokenSet {
            access_token: SecretString::new("a"),
            refresh_token: None,
            expires_at_ms: now_ms() + 30_000,
            token_endpoint: "https://t".into(),
            username: "u".into(),
        };
        assert!(set.is_expiring());
    }

    #[tokio::test]
    async fn store_round_trip_hides_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(EncryptedFileStore::new(dir.path().join("c.enc")));
        let mgr = AuthManager::new(reqwest::Client::new(), store.clone());
        assert!(!mgr.restore().await.unwrap());
        let set = TokenSet {
            access_token: SecretString::new("bearer-xyz"),
            refresh_token: Some(SecretString::new("refresh-xyz")),
            expires_at_ms: now_ms() + 3_600_000,
            token_endpoint: "https://id.example/token".into(),
            username: "alice".into(),
        };
        mgr.persist(&set).unwrap();
        let mgr2 = AuthManager::new(reqwest::Client::new(), store);
        assert!(mgr2.restore().await.unwrap());
        assert_eq!(mgr2.access_token().await.unwrap().expose(), "bearer-xyz");
        assert_eq!(mgr2.username().await.as_deref(), Some("alice"));
        mgr2.logout().await.unwrap();
        assert!(!mgr2.is_logged_in().await);
    }

    #[test]
    fn redaction_never_echoes_a_token() {
        let body = r#"{"access_token":"secret-value","error":"x"}"#;
        assert!(!redact_body(body).contains("secret-value"));
    }
}
