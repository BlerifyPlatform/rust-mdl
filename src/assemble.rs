//! `PUT /api/v1/organizations/{org}/projects/{project}/credentials/{cid}/sign`
//! — combine a previously-generated unsigned credential with an
//! externally-produced ES256 signature over its `signingMessage`, returning
//! the final ISO 18013-5 mdoc.
//!
//! The URL path is misleading: it ends in `/sign` but the operation is
//! "assemble" (the server merges credential + signature into the final
//! mdoc). Server-side test signing lives at `/crypto/sign/es256` and is
//! intentionally out of scope for this library — production issuers sign
//! locally with HSM/KMS.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

// ---------- request types ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleRequest {
    pub template_id: String,
    /// 128 lowercase hex chars — raw `r || s` (PLAIN format) for an ES256
    /// signature over the `signingMessage` returned by
    /// [`crate::BlerifyClient::generate`].
    pub signature: String,
    pub kid: String,
    /// PEM-encoded issuer certificate. Newlines must be preserved (`\n`).
    pub certificate: String,
}

// ---------- response types ----------

#[derive(Debug, Clone, Deserialize)]
pub struct AssembleResponse {
    /// Hex-encoded CBOR mdoc per ISO 18013-5 (lowercase, no separators).
    /// First byte is `0xa3` = CBOR map of three (`status`, `version`,
    /// `documents`). Decode with `hex::decode` to obtain raw bytes.
    pub mdoc: String,
}

// ---------- endpoint method ----------

impl BlerifyClient {
    /// Assemble the final mdoc by combining the server-side unsigned
    /// credential (identified by `credential_id`) with the caller-produced
    /// signature over its `signingMessage`.
    ///
    /// `credential_id` is the `0x`-prefixed identifier returned by
    /// [`BlerifyClient::generate`] — pass it through verbatim, the path
    /// preserves the prefix.
    #[instrument(
        skip_all,
        fields(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            template_id = %request.template_id,
            credential_id,
            correlation_id = ?correlation_id,
        ),
    )]
    pub async fn assemble(
        &self,
        credential_id: &str,
        request: &AssembleRequest,
        correlation_id: Option<Uuid>,
    ) -> Result<AssembleResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/sign",
            self.project_base_path(),
            credential_id,
        );
        debug!(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            template_id = %request.template_id,
            credential_id,
            "assemble credential",
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

    fn sample_request() -> AssembleRequest {
        AssembleRequest {
            template_id: "ca214a52-2291-4ad6-9b87-3d8fe988b0cc".into(),
            signature: "0".repeat(128),
            kid: "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ".into(),
            certificate: "-----BEGIN CERTIFICATE-----\nMII...\n-----END CERTIFICATE-----".into(),
        }
    }

    #[test]
    fn request_serializes_with_camelcase_keys() {
        let req = sample_request();
        let json = serde_json::to_value(&req).unwrap();

        assert!(
            json.get("templateId").is_some(),
            "templateId must be camelCase"
        );
        assert!(json.get("signature").is_some());
        assert!(json.get("kid").is_some());
        assert!(json.get("certificate").is_some());
        // signatureType is intentionally NOT sent on the wire — server defaults.
        assert!(
            json.get("signatureType").is_none(),
            "signatureType must not be sent"
        );
    }

    #[test]
    fn request_signature_is_128_hex_chars() {
        let req = sample_request();
        assert_eq!(
            req.signature.len(),
            128,
            "ES256 signature in PLAIN format is 128 hex chars"
        );
    }

    #[test]
    fn response_parses_real_wire_shape() {
        // Trimmed from the live capture against api.demo.blerify.com.
        let raw = r#"{"mdoc": "a366737461747573006776657273696f6e63312e30"}"#;
        let resp: AssembleResponse = serde_json::from_str(raw).expect("parses");
        assert!(
            resp.mdoc.starts_with("a3"),
            "mdoc starts with CBOR map-of-3 marker"
        );
        assert!(
            resp.mdoc.chars().all(|c| c.is_ascii_hexdigit()),
            "mdoc must be lowercase hex",
        );
    }
}
