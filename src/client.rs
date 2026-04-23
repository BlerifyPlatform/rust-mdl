use reqwest::Client;
use crate::auth::get_access_token;

pub struct BlerifyClient {
    pub base_url: String,
    pub client: Client,
    pub access_token: String,
}

impl BlerifyClient {
    pub fn new(base_url: String, access_token: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            access_token,
        }
    }

    pub async fn new_with_auth(base_url: String) -> Result<Self, reqwest::Error> {
        let token = get_access_token().await?;

        Ok(Self {
            base_url,
            client: Client::new(),
            access_token: token,
        })
    }
}