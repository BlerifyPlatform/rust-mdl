//! Live integration test against a real Blerify Issuance API instance.
//!
//! Disabled by default. Run with:
//!
//! ```bash
//! BLERIFY_RUN_LIVE_TESTS=1 \
//! BLERIFY_CREDS_PATH=/path/to/credentials.json \
//! BLERIFY_PROJECT_ID=<project-uuid> \
//! BLERIFY_CREDENTIAL_ID=<0x-credential-id> \
//! BLERIFY_BASE_URL=https://api.demo.blerify.com  # optional, default
//! cargo test --test live_status -- --nocapture
//! ```
//!
//! Reads the status of an existing credential. CI does not set
//! `BLERIFY_RUN_LIVE_TESTS` so the test is skipped.

use rust_mdl::{BlerifyClient, ServiceAccountCredentials};

const FLAG_ENV: &str = "BLERIFY_RUN_LIVE_TESTS";
const CREDS_PATH_ENV: &str = "BLERIFY_CREDS_PATH";
const PROJECT_ID_ENV: &str = "BLERIFY_PROJECT_ID";
const CREDENTIAL_ID_ENV: &str = "BLERIFY_CREDENTIAL_ID";
const BASE_URL_ENV: &str = "BLERIFY_BASE_URL";
const DEFAULT_BASE_URL: &str = "https://api.demo.blerify.com";

fn require(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set when running live tests"))
}

#[tokio::test]
async fn status_of_an_existing_credential() {
    if std::env::var(FLAG_ENV).is_err() {
        eprintln!("skipping: {FLAG_ENV} not set");
        return;
    }

    let creds =
        ServiceAccountCredentials::from_file(require(CREDS_PATH_ENV)).expect("load credentials");
    let base_url = std::env::var(BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into());
    let client = BlerifyClient::new(base_url, creds, require(PROJECT_ID_ENV));
    let credential_id = require(CREDENTIAL_ID_ENV);

    let response = client
        .status(&credential_id, None)
        .await
        .expect("status succeeds");

    assert_eq!(response.credential_id, credential_id);
    eprintln!(
        "status: {:?} revoked_at: {:?}",
        response.status, response.revoked_at
    );
}
