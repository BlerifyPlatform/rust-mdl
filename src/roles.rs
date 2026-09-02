use reqwest::Method;
use serde::Deserialize;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

/// One API role granted to the calling service account.
///
/// `project_id`/`project_name` are set for project-scoped roles (e.g.
/// `credentials.api`) and `None` for organization-level roles (e.g.
/// `notifications.api`). `project_name` can also be `None` when the project
/// has since been deleted.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountRole {
    pub name: String,

    #[serde(rename = "projectId", default)]
    pub project_id: Option<String>,

    #[serde(rename = "projectName", default)]
    pub project_name: Option<String>,
}

impl BlerifyClient {
    /// `GET /api/v1/iam/serviceAccounts/me/roles` — the API roles granted to
    /// the service account this client authenticates as.
    ///
    /// The caller's identity is derived server-side from the access token, so
    /// no parameters are sent: a service account can only ever read its own
    /// roles.
    #[instrument(skip_all)]
    pub async fn get_own_roles(
        &self,
        correlation_id: Option<Uuid>,
    ) -> Result<Vec<ServiceAccountRole>, BlerifyError> {
        let path = "/api/v1/iam/serviceAccounts/me/roles";

        debug!("get own service account roles");

        let response = self
            .request(Method::GET, path, correlation_id)
            .await?
            .send()
            .await?;

        decode_json_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_project_and_organization_scoped_roles() {
        let body = r#"[
            {
                "name": "credentials.api",
                "projectId": "d56624f1-1b32-4fbd-9156-9d108cf5a244",
                "projectName": "My Project"
            },
            { "name": "notifications.api", "projectId": null, "projectName": null }
        ]"#;

        let roles: Vec<ServiceAccountRole> = serde_json::from_str(body).unwrap();
        assert_eq!(roles.len(), 2);

        assert_eq!(roles[0].name, "credentials.api");
        assert_eq!(
            roles[0].project_id.as_deref(),
            Some("d56624f1-1b32-4fbd-9156-9d108cf5a244")
        );
        assert_eq!(roles[0].project_name.as_deref(), Some("My Project"));

        assert_eq!(roles[1].name, "notifications.api");
        assert!(roles[1].project_id.is_none());
        assert!(roles[1].project_name.is_none());
    }

    #[test]
    fn tolerates_responses_without_the_project_name_field() {
        // Older backend versions return only name/projectId.
        let body = r#"[{ "name": "credentials.api", "projectId": "abc" }]"#;

        let roles: Vec<ServiceAccountRole> = serde_json::from_str(body).unwrap();
        assert_eq!(roles[0].project_id.as_deref(), Some("abc"));
        assert!(roles[0].project_name.is_none());
    }
}
