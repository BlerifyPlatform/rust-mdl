//! `GET /api/v1/organizations/{org}/projects/{project}/credentials/{cid}/status`
//! — read whether a credential can still be trusted.
//!
//! The server reads the on-chain verification registry, so the answer
//! reflects revocations and holds once they are recorded on chain (a revoke
//! returns `202` before that happens). A `503` means the registry could not
//! be read; the server never answers `VALID` in that case.

use reqwest::Method;
use serde::Deserialize;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

// ---------- response types ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub credential_id: String,
    pub status: CredentialStatus,
    /// Only present when `status` is [`CredentialStatus::Revoked`]. ISO-8601
    /// date-time in UTC without an offset, as the server sends it; kept as a
    /// string so a format change on the server doesn't fail the whole read.
    #[serde(default)]
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialStatus {
    Valid,
    Revoked,
    Suspended,
    Expired,
    /// Not registered on chain yet.
    Pending,
    /// A value this version of the library doesn't know. Treat it as not
    /// valid.
    #[serde(other)]
    Unknown,
}

// ---------- endpoint method ----------

impl BlerifyClient {
    /// Read the lifecycle status of a credential.
    ///
    /// `credential_id` is the `0x`-prefixed identifier returned by
    /// [`BlerifyClient::generate`] — preserved verbatim into the URL path.
    #[instrument(
        skip_all,
        fields(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            credential_id,
            correlation_id = ?correlation_id,
        ),
    )]
    pub async fn status(
        &self,
        credential_id: &str,
        correlation_id: Option<Uuid>,
    ) -> Result<StatusResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/status",
            self.project_base_path(),
            credential_id,
        );
        debug!(credential_id, "read credential status");

        let response = self
            .request(Method::GET, &path, correlation_id)
            .await?
            .send()
            .await?;

        decode_json_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0x5fbfb17571f323dc3b1bfcd30fdb2bd785d3d61738f4f75da6853d770a6f43da";

    #[test]
    fn parses_valid_without_revocation_date() {
        let raw = format!(r#"{{"credentialId":"{ID}","status":"VALID"}}"#);
        let resp: StatusResponse = serde_json::from_str(&raw).expect("parses");
        assert_eq!(resp.credential_id, ID);
        assert_eq!(resp.status, CredentialStatus::Valid);
        assert!(resp.revoked_at.is_none());
    }

    #[test]
    fn parses_revoked_with_revocation_date() {
        let raw = format!(
            r#"{{"credentialId":"{ID}","status":"REVOKED","revokedAt":"2026-10-08T13:38:10"}}"#
        );
        let resp: StatusResponse = serde_json::from_str(&raw).expect("parses");
        assert_eq!(resp.status, CredentialStatus::Revoked);
        assert_eq!(resp.revoked_at.as_deref(), Some("2026-10-08T13:38:10"));
    }

    #[test]
    fn parses_every_known_status() {
        for (wire, expected) in [
            ("SUSPENDED", CredentialStatus::Suspended),
            ("EXPIRED", CredentialStatus::Expired),
            ("PENDING", CredentialStatus::Pending),
        ] {
            let raw = format!(r#"{{"credentialId":"{ID}","status":"{wire}"}}"#);
            let resp: StatusResponse = serde_json::from_str(&raw).expect("parses");
            assert_eq!(resp.status, expected, "{wire}");
        }
    }

    #[test]
    fn unknown_status_does_not_fail_the_read() {
        let raw = format!(r#"{{"credentialId":"{ID}","status":"SOMETHING_NEW"}}"#);
        let resp: StatusResponse = serde_json::from_str(&raw).expect("parses");
        assert_eq!(resp.status, CredentialStatus::Unknown);
    }
}
