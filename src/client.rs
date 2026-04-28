use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, RequestBuilder};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::auth::TokenManager;
use crate::credentials::ServiceAccountCredentials;
use crate::error::BlerifyError;

/// TCP connect deadline for outbound API calls.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Total per-request deadline (connect + headers + body).
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Build a [`reqwest::Client`] with the library's default timeout configuration.
///
/// Used by [`BlerifyClient`] and [`crate::auth::TokenManager`] when callers
/// don't supply their own client. Override by constructing with
/// [`reqwest::Client::builder`] and passing the result to
/// [`BlerifyClient::from_parts`] / [`crate::auth::TokenManager::with_client`].
pub fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
        .timeout(DEFAULT_REQUEST_TIMEOUT)
        .build()
        .expect("default reqwest client builds with system TLS")
}

/// HTTP client for Blerify's Issuance API.
///
/// Holds the long-lived state required to issue API calls scoped to a single
/// `(organization, project)` pair: the base URL, an auth token cache, and a
/// shared `reqwest::Client`. Endpoint methods (`generate`, `assemble`, …)
/// live in their own modules and extend this type via `impl` blocks.
#[derive(Clone)]
pub struct BlerifyClient {
    base_url: String,
    org_id: String,
    project_id: String,
    http: reqwest::Client,
    tokens: Arc<TokenManager>,
}

impl BlerifyClient {
    /// Construct a client from service-account credentials and a project ID.
    /// The organisation ID is taken from `credentials.organization_id`.
    pub fn new(
        base_url: impl Into<String>,
        credentials: ServiceAccountCredentials,
        project_id: impl Into<String>,
    ) -> Self {
        let org_id = credentials.organization_id.clone();
        let http = default_http_client();
        Self {
            base_url: base_url.into(),
            org_id,
            project_id: project_id.into(),
            http: http.clone(),
            tokens: Arc::new(TokenManager::with_client(credentials, http)),
        }
    }

    /// Construct a client backed by a caller-managed [`TokenManager`].
    /// Useful when sharing one token cache across multiple project clients.
    pub fn from_token_manager(
        base_url: impl Into<String>,
        tokens: Arc<TokenManager>,
        org_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            org_id: org_id.into(),
            project_id: project_id.into(),
            http: default_http_client(),
            tokens,
        }
    }

    /// Construct from caller-supplied parts. The `http` client is used for
    /// API calls; the `TokenManager`'s internal client is used for auth.
    pub fn from_parts(
        base_url: impl Into<String>,
        tokens: Arc<TokenManager>,
        http: reqwest::Client,
        org_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            org_id: org_id.into(),
            project_id: project_id.into(),
            http,
            tokens,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn org_id(&self) -> &str {
        &self.org_id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// `/api/v1/organizations/{org}/projects/{project}` — common prefix for
    /// every credential-related endpoint on the Issuance API.
    pub(crate) fn project_base_path(&self) -> String {
        format!(
            "/api/v1/organizations/{}/projects/{}",
            self.org_id, self.project_id,
        )
    }

    /// Build a request to `path` (relative to `base_url`) with the auth token,
    /// `Content-Type: application/json`, and `correlation-id` header set.
    /// Callers chain `.json(&body)` / `.send()` as usual.
    pub(crate) async fn request(
        &self,
        method: Method,
        path: &str,
        correlation_id: Option<Uuid>,
    ) -> Result<RequestBuilder, BlerifyError> {
        let token = self.tokens.access_token().await?;
        let url = format!("{}{}", self.base_url, path);
        let cid = correlation_id.unwrap_or_else(Uuid::new_v4);

        Ok(self
            .http
            .request(method, &url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .header("correlation-id", cid.to_string()))
    }
}

/// Decode a JSON response, mapping non-2xx into [`BlerifyError::Server`].
///
/// Used by every endpoint method in this crate to keep error envelope
/// handling consistent.
pub(crate) async fn decode_json_response<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, BlerifyError> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .json::<serde_json::Value>()
            .await
            .unwrap_or(serde_json::Value::Null);
        let message = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Err(BlerifyError::Server {
            status: status.as_u16(),
            message,
            body,
        });
    }
    Ok(response.json::<T>().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_creds() -> ServiceAccountCredentials {
        ServiceAccountCredentials {
            client_id: "client-id".into(),
            organization_id: "org-uuid".into(),
            token_uri: "https://example.invalid/token".into(),
            iam_audience: "aud".into(),
            private_key: "fake".into(),
            client_email: None,
            private_key_id: None,
        }
    }

    #[test]
    fn project_base_path_is_well_formed() {
        let client = BlerifyClient::new("https://api.demo.blerify.com", fake_creds(), "proj-1");
        assert_eq!(
            client.project_base_path(),
            "/api/v1/organizations/org-uuid/projects/proj-1"
        );
        assert_eq!(client.org_id(), "org-uuid");
        assert_eq!(client.project_id(), "proj-1");
        assert_eq!(client.base_url(), "https://api.demo.blerify.com");
    }
}
