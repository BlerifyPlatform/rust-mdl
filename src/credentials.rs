use std::path::Path;

use serde::Deserialize;

use crate::error::BlerifyError;

/// Service-account credentials JSON downloaded from the Blerify portal.
///
/// The shape mirrors a Google service-account file, but the auth flow itself
/// is custom (see [`crate::auth`]). Only the fields required to mint a
/// `client_assertion` JWT and call the token endpoint are deserialised
/// strictly; the rest are tolerated for forward-compat.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountCredentials {
    pub client_id: String,
    pub organization_id: String,
    pub token_uri: String,
    pub iam_audience: String,
    pub private_key: String,

    #[serde(default)]
    pub client_email: Option<String>,
    #[serde(default)]
    pub private_key_id: Option<String>,
}

impl ServiceAccountCredentials {
    /// Load credentials from a JSON file at `path`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, BlerifyError> {
        let raw = std::fs::read_to_string(path)?;
        let creds: Self = serde_json::from_str(&raw)?;
        Ok(creds)
    }

    /// Parse credentials from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, BlerifyError> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "type": "service_account",
        "organization_id": "69a4f65f-1129-4e64-8a4b-1c299088bf89",
        "private_key_id": "key-id-123",
        "client_email": "sa@example.blerify.com",
        "client_id": "client-id-abc",
        "token_uri": "https://api.demo.blerify.com/auth/v2/protocol/openid-connect/token",
        "auth_provider_x509_cert_url": "https://api.demo.blerify.com/auth/v2/{org}/protocol/openid-connect/v1/certs",
        "iam_audience": "https://iam.demo.blerify.com/realms/{org}",
        "universe_domain": "blerify.com",
        "private_key": "-----BEGIN PRIVATE KEY-----\nFAKE\n-----END PRIVATE KEY-----"
    }"#;

    #[test]
    fn parses_sample_credentials_json() {
        let creds = ServiceAccountCredentials::from_json(SAMPLE).expect("parse");
        assert_eq!(creds.client_id, "client-id-abc");
        assert_eq!(
            creds.organization_id,
            "69a4f65f-1129-4e64-8a4b-1c299088bf89"
        );
        assert!(creds.token_uri.starts_with("https://"));
        assert!(creds.iam_audience.contains("realms"));
        assert!(creds.private_key.starts_with("-----BEGIN"));
        assert_eq!(
            creds.client_email.as_deref(),
            Some("sa@example.blerify.com")
        );
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let bad = r#"{"client_id":"x","organization_id":"y","token_uri":"z","iam_audience":"a"}"#;
        // private_key missing → parse error
        assert!(ServiceAccountCredentials::from_json(bad).is_err());
    }
}
