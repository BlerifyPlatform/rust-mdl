use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

#[derive(Debug, Clone, Serialize)]
pub struct ValidateRequest {
    pub mdoc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateResponse {
    #[serde(default)]
    pub status: i32,

    #[serde(rename = "verifyInfo", default)]
    pub verify_info: Value,

    #[serde(flatten)]
    pub extra: Value,
}

impl BlerifyClient {
    #[instrument(skip_all)]
    pub async fn validate(
        &self,
        credential_id: &str,
        request: &ValidateRequest,
        correlation_id: Option<Uuid>,
    ) -> Result<ValidateResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/signature/validate",
            self.project_base_path(),
            credential_id
        );

        debug!("validate credential");

        let response = self
            .request(Method::POST, &path, correlation_id)
            .await?
            .json(request)
            .send()
            .await?;

        decode_json_response(response).await
    }
}
