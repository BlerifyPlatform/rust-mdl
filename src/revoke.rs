//! `PUT /api/v1/organizations/{org}/projects/{project}/credentials/{cid}/revoke`
//! — irreversibly revoke a previously-issued credential.
//!
//! Returns `202 Accepted` with a body that just echoes the `correlation-id`
//! header back. The actual revocation status is reflected by subsequent
//! reads of the credential (out of scope for this library).

use reqwest::Method;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

// ---------- request types ----------

#[derive(Debug, Clone, Serialize)]
pub struct RevokeRequest {
    pub metadata: StateChangeMetadata,
}

/// Metadata captured against any credential state change (revoke, hold, …).
#[derive(Debug, Clone, Serialize)]
pub struct StateChangeMetadata {
    pub code: String,
    pub description: String,
    pub category: String,
}

// ---------- response types ----------

/// The server returns `202 Accepted` with only the request's correlation-id
/// echoed in the body. No other fields are guaranteed.
#[derive(Debug, Clone, Deserialize)]
pub struct RevokeResponse {
    #[serde(rename = "correlation-id", default)]
    pub correlation_id: Option<String>,
}

// ---------- endpoint method ----------

impl BlerifyClient {
    /// Irreversibly revoke a credential.
    ///
    /// `credential_id` is the `0x`-prefixed identifier returned by
    /// [`BlerifyClient::generate`] / [`BlerifyClient::assemble`] — preserved
    /// verbatim into the URL path.
    #[instrument(
        skip_all,
        fields(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            credential_id,
            reason_code = %request.metadata.code,
            correlation_id = ?correlation_id,
        ),
    )]
    pub async fn revoke(
        &self,
        credential_id: &str,
        request: &RevokeRequest,
        correlation_id: Option<Uuid>,
    ) -> Result<RevokeResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/revoke",
            self.project_base_path(),
            credential_id,
        );
        debug!(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            credential_id,
            "revoke credential",
        );

        let response = self
            .request(Method::PUT, &path, correlation_id)
            .await?
            .json(request)
            .send()
            .await?;

        decode_json_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> RevokeRequest {
        RevokeRequest {
            metadata: StateChangeMetadata {
                code: "DEVICE_LOST".into(),
                description: "Citizen reported device lost".into(),
                category: "device".into(),
            },
        }
    }

    #[test]
    fn request_serializes_with_metadata_envelope() {
        let req = sample_request();
        let json = serde_json::to_value(&req).unwrap();

        assert!(
            json.get("metadata").is_some(),
            "metadata is the only top-level key"
        );
        let meta = &json["metadata"];
        assert_eq!(meta["code"], "DEVICE_LOST");
        assert_eq!(meta["description"], "Citizen reported device lost");
        assert_eq!(meta["category"], "device");
    }

    #[test]
    fn response_parses_correlation_id_only_body() {
        // Live capture wire shape — server returns just the echoed correlation-id.
        let raw = r#"{"correlation-id":"08bfa092-c3df-4e3a-8c4c-520c5e0d88fa"}"#;
        let resp: RevokeResponse = serde_json::from_str(raw).expect("parses");
        assert_eq!(
            resp.correlation_id.as_deref(),
            Some("08bfa092-c3df-4e3a-8c4c-520c5e0d88fa"),
        );
    }

    #[test]
    fn response_parses_empty_body() {
        // Defensive — accept an empty body in case the server simplifies later.
        let raw = r#"{}"#;
        let resp: RevokeResponse = serde_json::from_str(raw).expect("parses");
        assert!(resp.correlation_id.is_none());
    }
}
