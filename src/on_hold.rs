use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;
use crate::revoke::StateChangeMetadata;

#[derive(Debug, Clone, Deserialize)]
pub struct OnHoldResponse {
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnHoldRequest {
    pub status: bool,
    pub metadata: StateChangeMetadata,
}

impl BlerifyClient {
    #[instrument(skip_all)]
    pub async fn on_hold(
        &self,
        credential_id: &str,
        request: &OnHoldRequest,
        correlation_id: Option<Uuid>,
    ) -> Result<OnHoldResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/hold",
            self.project_base_path(),
            credential_id
        );

        debug!("onHold credential");

        let response = self
            .request(Method::PUT, &path, correlation_id)
            .await?
            .json(request)
            .send()
            .await?;

        decode_json_response(response).await
    }
}
