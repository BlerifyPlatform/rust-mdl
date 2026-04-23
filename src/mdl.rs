use crate::client::BlerifyClient;

impl BlerifyClient {
    pub async fn get_document(&self) -> Result<String, reqwest::Error> {
        let url = format!("{}/mdl/document", self.base_url);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let text = response.text().await?;

        Ok(text)
    }
}