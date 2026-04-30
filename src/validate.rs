use reqwest::Method;
use serde::Deserialize;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateResponse {
    #[serde(default)]
    pub valid: Option<bool>,

    #[serde(default)]
    pub status: Option<String>,
}

impl BlerifyClient {
    #[instrument(skip_all)]
    pub async fn validate(
        &self,
        credential_id: &str,
        correlation_id: Option<Uuid>,
    ) -> Result<ValidateResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/validate",
            self.project_base_path(),
            credential_id
        );

        debug!("validate credential");

        let response = self
            .request(Method::GET, &path, correlation_id)
            .await?
            .send()
            .await?;

        decode_json_response(response).await
    }
}
