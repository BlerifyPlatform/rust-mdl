//! `POST /api/v1/organizations/{org}/projects/{project}/credentials` — create
//! an unsigned mDL credential and receive the `signingMessage` to sign
//! externally before calling [`crate::BlerifyClient::assemble`].

use reqwest::Method;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

// ---------- request types ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub template_id: String,
    pub additional_data: AdditionalData,
    pub organization_user: OrganizationUser,
    pub options: Options,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    pub mdl_data: MdlData,
    pub validity_info: ValidityInfo,
    pub device_public_key: JwkP256,
    /// PEM-encoded issuer certificate. Newlines must be preserved (`\n`).
    pub certificate: String,
    pub kid: String,
    #[serde(default)]
    pub namespaces: Vec<NamespaceEntry>,
}

/// mDL data elements per ISO/IEC 18013-5:2021 §7.2.1 Table 5.
///
/// Mandatory (M) fields are non-`Option`; optional (O) fields are `Option<T>`
/// and skipped from the wire when `None`. Use [`MdlData::new`] to construct
/// with only the mandatory fields, then assign optional fields directly:
///
/// ```ignore
/// let mut data = MdlData::new(
///     "Doe", "John", "1987-03-15", "2025-10-15", "2028-09-30",
///     "US", "Acme", "8-203-1365",
///     /* portrait JPEG hex */ "FFD8FF...".to_string(),
///     vec![DrivingPrivilege { /* ... */ }],
///     "PA",
/// );
/// data.nationality = Some("PA".into());
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MdlData {
    // ---- Mandatory (per ISO 18013-5 §7.2.1 Table 5) ----
    pub family_name: String,
    pub given_name: String,
    /// Day, month and year of birth — `full-date` (YYYY-MM-DD).
    pub birth_date: String,
    /// `tdate` or `full-date` — date the mDL was issued.
    pub issue_date: String,
    /// `tdate` or `full-date` — date the mDL expires.
    pub expiry_date: String,
    /// Alpha-2 country code (ISO 3166-1) of the issuing authority.
    pub issuing_country: String,
    pub issuing_authority: String,
    pub document_number: String,
    /// Hex-encoded JPEG/JPEG2000 bytes (uppercase or lowercase). Per
    /// ISO 18013-2:2020 Annex D for the portrait image format.
    pub portrait: String,
    pub driving_privileges: Vec<DrivingPrivilege>,
    /// Distinguishing sign per ISO/IEC 18013-1:2018 Annex F.
    pub un_distinguishing_sign: String,
    // ---- Optional (per ISO 18013-5 §7.2.1 Table 5) ----
    #[serde(skip_serializing_if = "Option::is_none")]
    pub administrative_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eye_colour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_place: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuing_jurisdiction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nationality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_country: Option<String>,
    /// Hex-encoded JPEG bytes (uppercase or lowercase).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_usual_mark: Option<String>,
}

impl MdlData {
    /// Construct with the 11 mandatory fields per ISO 18013-5 §7.2.1 Table 5.
    /// All optional fields default to `None`; assign them directly afterwards.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        family_name: impl Into<String>,
        given_name: impl Into<String>,
        birth_date: impl Into<String>,
        issue_date: impl Into<String>,
        expiry_date: impl Into<String>,
        issuing_country: impl Into<String>,
        issuing_authority: impl Into<String>,
        document_number: impl Into<String>,
        portrait_hex: impl Into<String>,
        driving_privileges: Vec<DrivingPrivilege>,
        un_distinguishing_sign: impl Into<String>,
    ) -> Self {
        Self {
            family_name: family_name.into(),
            given_name: given_name.into(),
            birth_date: birth_date.into(),
            issue_date: issue_date.into(),
            expiry_date: expiry_date.into(),
            issuing_country: issuing_country.into(),
            issuing_authority: issuing_authority.into(),
            document_number: document_number.into(),
            portrait: portrait_hex.into(),
            driving_privileges,
            un_distinguishing_sign: un_distinguishing_sign.into(),
            administrative_number: None,
            sex: None,
            height: None,
            weight: None,
            eye_colour: None,
            birth_place: None,
            resident_address: None,
            issuing_jurisdiction: None,
            nationality: None,
            resident_city: None,
            resident_state: None,
            resident_postal_code: None,
            resident_country: None,
            signature_usual_mark: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DrivingPrivilege {
    pub vehicle_category_code: String,
    pub issue_date: String,
    pub expiry_date: String,
    pub codes: Vec<DrivingCode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DrivingCode {
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidityInfo {
    /// ISO 8601 timestamp with `Z` suffix.
    pub signed: String,
    pub valid_from: String,
    pub valid_until: String,
}

/// JWK for an EC P-256 public key. `x` and `y` are base64url-encoded
/// (RFC 4648 §5, no padding) — **not** standard base64.
#[derive(Debug, Clone, Serialize)]
pub struct JwkP256 {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub y: String,
}

impl JwkP256 {
    /// Convenience constructor for an EC P-256 key.
    pub fn ec_p256(x: impl Into<String>, y: impl Into<String>) -> Self {
        Self {
            kty: "EC".into(),
            crv: "P-256".into(),
            x: x.into(),
            y: y.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NamespaceEntry {
    pub title: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrganizationUser {
    pub id: String,
    pub did: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// Server-side flag — must be `true` for mDL credentials. Default: `true`.
    pub additional_data: bool,
    pub onboard: bool,
    pub update: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            additional_data: true,
            onboard: false,
            update: false,
        }
    }
}

// ---------- response types ----------

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub credential: GeneratedCredential,
    /// Base64url-encoded COSE `Signature1` ToBeSigned bytes. Sign with ES256
    /// to produce the signature for [`crate::BlerifyClient::assemble`].
    #[serde(rename = "signingMessage")]
    pub signing_message: String,
}

/// Subset of the `credential` object returned by the server. Only `id` is
/// strictly required — other fields are surfaced for callers that want them
/// but the server may add more in the future (the type silently ignores
/// unknown fields).
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedCredential {
    /// Credential identifier as emitted by the server — `0x` + 64 hex chars.
    /// Pass through verbatim to [`crate::BlerifyClient::assemble`].
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub template: Option<TemplateInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
}

// ---------- endpoint method ----------

impl BlerifyClient {
    /// Create an unsigned mDL credential.
    ///
    /// `correlation_id` is forwarded as the `correlation-id` HTTP header. If
    /// `None`, a fresh UUIDv4 is generated.
    ///
    /// Returns the server-assigned credential id (with `0x` prefix) and the
    /// `signingMessage` (base64url-encoded ToBeSigned bytes).
    pub async fn generate(
        &self,
        request: &GenerateRequest,
        correlation_id: Option<Uuid>,
    ) -> Result<GenerateResponse, BlerifyError> {
        let path = format!("{}/credentials", self.project_base_path());
        debug!(
            org_id = %self.org_id(),
            project_id = %self.project_id(),
            template_id = %request.template_id,
            "generate credential",
        );

        let response = self
            .request(Method::POST, &path, correlation_id)
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

    fn sample_request() -> GenerateRequest {
        let mut mdl_data = MdlData::new(
            "Maravi",
            "Washington",
            "1987-03-15",
            "2025-10-15",
            "2028-09-30",
            "US",
            "Acme",
            "8-203-1365",
            "FFD8FF",
            vec![DrivingPrivilege {
                vehicle_category_code: "C".into(),
                issue_date: "2025-08-25".into(),
                expiry_date: "2028-09-30".into(),
                codes: vec![DrivingCode { code: "210".into() }],
            }],
            "PA",
        );
        mdl_data.nationality = Some("PA".into());

        GenerateRequest {
            template_id: "ca214a52-2291-4ad6-9b87-3d8fe988b0cc".into(),
            additional_data: AdditionalData {
                mdl_data,
                validity_info: ValidityInfo {
                    signed: "2025-10-28T10:10:18Z".into(),
                    valid_from: "2025-10-29T20:46:25Z".into(),
                    valid_until: "2030-02-13T10:10:18Z".into(),
                },
                device_public_key: JwkP256::ec_p256(
                    "iBh5ynojixm_D0wfjADpouGbp6b3Pq6SuFHU3htQhVk",
                    "oxS1OAORJ7XNUHNfVFGeM8E0RQVFxWA62fJj-sxW03c",
                ),
                certificate: "-----BEGIN CERTIFICATE-----\nMIIBZD...\n-----END CERTIFICATE-----"
                    .into(),
                kid: "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ".into(),
                namespaces: vec![],
            },
            organization_user: OrganizationUser {
                id: "8-203-1365".into(),
                did: "did:lac1:abc".into(),
            },
            options: Options {
                additional_data: true,
                onboard: false,
                update: true,
            },
        }
    }

    #[test]
    fn request_serializes_with_camelcase_top_level_and_snake_case_mdldata() {
        let req = sample_request();
        let json = serde_json::to_value(&req).unwrap();

        // Top-level keys: camelCase
        assert!(json.get("templateId").is_some());
        assert!(json.get("additionalData").is_some());
        assert!(json.get("organizationUser").is_some());
        assert!(json.get("options").is_some());

        // additionalData keys: camelCase
        let additional = &json["additionalData"];
        assert!(additional.get("mdlData").is_some());
        assert!(additional.get("validityInfo").is_some());
        assert!(additional.get("devicePublicKey").is_some());
        assert!(additional.get("certificate").is_some());
        assert!(additional.get("kid").is_some());

        // mdlData keys: snake_case
        let mdl = &additional["mdlData"];
        assert_eq!(mdl["family_name"], "Maravi");
        assert_eq!(mdl["given_name"], "Washington");
        assert_eq!(mdl["birth_date"], "1987-03-15");

        // validityInfo keys: camelCase
        let validity = &additional["validityInfo"];
        assert!(validity.get("signed").is_some());
        assert!(validity.get("validFrom").is_some());
        assert!(validity.get("validUntil").is_some());

        // options keys: camelCase
        assert_eq!(json["options"]["additionalData"], true);
    }

    #[test]
    fn request_omits_none_mdldata_fields() {
        let req = sample_request();
        let json = serde_json::to_value(&req).unwrap();
        let mdl = &json["additionalData"]["mdlData"];

        // We didn't set these — they must NOT appear on the wire.
        for absent in [
            "administrative_number",
            "sex",
            "height",
            "eye_colour",
            "resident_address",
        ] {
            assert!(mdl.get(absent).is_none(), "{absent} should be omitted");
        }
    }

    #[test]
    fn options_default_matches_phpmdl_behavior() {
        let opts = Options::default();
        assert!(opts.additional_data);
        assert!(!opts.onboard);
        assert!(!opts.update);
    }

    #[test]
    fn response_parses_real_wire_shape() {
        // Trimmed from the live capture against api.demo.blerify.com on 2026-04-28.
        let raw = r#"{
            "credential": {
                "_id": "0xd5e564ab9975cf1bc69d453123e2601fb222277549adba93149776a54e787caf",
                "issuer": "did:lac1:1APfzphXuZ1wSZkvisXswFaqZdZaJSLEBKbQcTnysxu16vgJvebB3AWXaeSELhahtXR",
                "status": "PENDING",
                "template": {
                    "id": "ca214a52-2291-4ad6-9b87-3d8fe988b0cc",
                    "projectId": "057bd751-5bf6-4f98-9b75-9ec284150709",
                    "organizationId": "69a4f65f-1129-4e64-8a4b-1c299088bf89",
                    "network": "AVALANCHE"
                },
                "data": {"iv": "...", "cipher": "..."},
                "issued": null
            },
            "signingMessage": "hGpTaWduYXR1cmUxQ6EBJk"
        }"#;

        let resp: GenerateResponse = serde_json::from_str(raw).expect("parses");
        assert!(resp.credential.id.starts_with("0x"));
        assert_eq!(resp.credential.id.len(), 66); // 0x + 64 hex
        assert_eq!(resp.credential.status.as_deref(), Some("PENDING"));
        assert!(resp
            .credential
            .issuer
            .as_deref()
            .unwrap()
            .starts_with("did:lac1:"));
        let tmpl = resp.credential.template.expect("template present");
        assert_eq!(tmpl.network.as_deref(), Some("AVALANCHE"));
        assert!(!resp.signing_message.is_empty());
    }
}
