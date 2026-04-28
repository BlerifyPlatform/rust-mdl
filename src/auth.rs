use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::default_http_client;
use crate::credentials::ServiceAccountCredentials;
use crate::error::BlerifyError;

const ASSERTION_TTL_SECS: u64 = 3600;
const REFRESH_SKEW_SECS: u64 = 60;
const DEFAULT_TOKEN_TTL_SECS: u64 = 3600;

#[derive(Debug, Serialize)]
struct AssertionClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: u64,
    exp: u64,
    jti: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    /// Unix timestamp (seconds) when the token expires.
    expires_at: u64,
}

/// Manages access-token lifecycle for the Blerify Issuance API.
///
/// On each call to [`TokenManager::access_token`], returns the cached token
/// if it is still valid for at least [`REFRESH_SKEW_SECS`] more seconds,
/// otherwise mints a fresh one by signing a `client_assertion` JWT and
/// exchanging it at the configured `token_uri`.
pub struct TokenManager {
    credentials: ServiceAccountCredentials,
    http: Client,
    cache: Mutex<Option<CachedToken>>,
}

impl TokenManager {
    pub fn new(credentials: ServiceAccountCredentials) -> Self {
        Self {
            credentials,
            http: default_http_client(),
            cache: Mutex::new(None),
        }
    }

    /// Construct with a caller-supplied [`reqwest::Client`] (useful for sharing
    /// a connection pool or installing custom middleware).
    pub fn with_client(credentials: ServiceAccountCredentials, http: Client) -> Self {
        Self {
            credentials,
            http,
            cache: Mutex::new(None),
        }
    }

    /// Returns a valid access token, refreshing if cache is stale.
    #[instrument(skip_all, fields(client_id = %self.credentials.client_id))]
    pub async fn access_token(&self) -> Result<String, BlerifyError> {
        let mut guard = self.cache.lock().await;
        let now = unix_now()?;

        if let Some(cached) = guard.as_ref() {
            if cached.expires_at > now + REFRESH_SKEW_SECS {
                return Ok(cached.access_token.clone());
            }
        }

        let fresh = self.mint_token(now).await?;
        let token = fresh.access_token.clone();
        *guard = Some(fresh);
        Ok(token)
    }

    async fn mint_token(&self, now: u64) -> Result<CachedToken, BlerifyError> {
        let assertion = self.build_assertion(now)?;

        let params = [
            ("client_id", self.credentials.client_id.as_str()),
            ("organization_id", self.credentials.organization_id.as_str()),
            ("client_assertion", assertion.as_str()),
        ];

        debug!(
            client_id = %self.credentials.client_id,
            token_uri = %self.credentials.token_uri,
            "minting access token",
        );

        let response = self
            .http
            .post(&self.credentials.token_uri)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?
            .error_for_status()?;

        let body: TokenResponse = response.json().await?;
        let ttl = body.expires_in.unwrap_or(DEFAULT_TOKEN_TTL_SECS);

        Ok(CachedToken {
            access_token: body.access_token,
            expires_at: now + ttl,
        })
    }

    fn build_assertion(&self, now: u64) -> Result<String, BlerifyError> {
        let claims = AssertionClaims {
            iss: self.credentials.client_id.clone(),
            sub: self.credentials.client_id.clone(),
            aud: self.credentials.iam_audience.clone(),
            iat: now,
            exp: now + ASSERTION_TTL_SECS,
            jti: Uuid::new_v4().to_string(),
        };

        let key = EncodingKey::from_rsa_pem(self.credentials.private_key.as_bytes())?;
        Ok(encode(
            &Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &key,
        )?)
    }
}

fn unix_now() -> Result<u64, BlerifyError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_returns_same_token_within_skew_window() {
        // Manually populate the cache and assert the read path doesn't refresh.
        let creds = ServiceAccountCredentials {
            client_id: "x".into(),
            organization_id: "y".into(),
            token_uri: "https://example.invalid/should-not-be-called".into(),
            iam_audience: "aud".into(),
            private_key: "fake".into(),
            client_email: None,
            private_key_id: None,
        };
        let mgr = TokenManager::new(creds);
        let now = unix_now().unwrap();
        *mgr.cache.lock().await = Some(CachedToken {
            access_token: "cached-abc".into(),
            expires_at: now + 600,
        });

        let token = mgr.access_token().await.expect("returns cached");
        assert_eq!(token, "cached-abc");
    }

    #[tokio::test]
    async fn cache_refreshes_when_expired() {
        // Stale cache + bogus token_uri → access_token() must try to mint and fail at the network step.
        let creds = ServiceAccountCredentials {
            client_id: "x".into(),
            organization_id: "y".into(),
            token_uri: "http://127.0.0.1:1/never".into(),
            iam_audience: "aud".into(),
            private_key: "-----BEGIN PRIVATE KEY-----\ninvalid\n-----END PRIVATE KEY-----".into(),
            client_email: None,
            private_key_id: None,
        };
        let mgr = TokenManager::new(creds);
        let now = unix_now().unwrap();
        *mgr.cache.lock().await = Some(CachedToken {
            access_token: "stale".into(),
            expires_at: now.saturating_sub(1),
        });

        // We expect a Jwt error (invalid private key) before we ever hit the network.
        // Either Jwt or Transport is acceptable — the point is it does NOT return "stale".
        let result = mgr.access_token().await;
        assert!(matches!(
            result,
            Err(BlerifyError::Jwt(_)) | Err(BlerifyError::Transport(_))
        ));
    }
}
