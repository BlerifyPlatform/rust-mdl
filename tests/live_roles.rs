//! Live integration test against a real Blerify API instance.
//!
//! Disabled by default. Run with:
//!
//! ```bash
//! BLERIFY_RUN_LIVE_TESTS=1 \
//! BLERIFY_CREDS_PATH=/path/to/credentials.json \
//! BLERIFY_PROJECT_ID=<project-uuid> \
//! BLERIFY_BASE_URL=https://api.demo.blerify.com  # optional, default
//! cargo test --test live_roles -- --nocapture
//! ```
//!
//! The test reads the roles of the configured service account through the
//! self-service endpoint. CI does not set `BLERIFY_RUN_LIVE_TESTS` so the
//! test is skipped.

use rust_mdl::{BlerifyClient, ServiceAccountCredentials};

const FLAG_ENV: &str = "BLERIFY_RUN_LIVE_TESTS";
const CREDS_PATH_ENV: &str = "BLERIFY_CREDS_PATH";
const PROJECT_ID_ENV: &str = "BLERIFY_PROJECT_ID";
const BASE_URL_ENV: &str = "BLERIFY_BASE_URL";
const DEFAULT_BASE_URL: &str = "https://api.demo.blerify.com";

fn skip_unless_live() -> Option<()> {
    if std::env::var(FLAG_ENV).is_err() {
        eprintln!("skipping: {FLAG_ENV} not set");
        return None;
    }
    Some(())
}

fn require(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set when running live tests"))
}

#[tokio::test]
async fn live_get_own_roles() {
    if skip_unless_live().is_none() {
        return;
    }

    let creds = ServiceAccountCredentials::from_file(require(CREDS_PATH_ENV))
        .expect("credentials file loads");
    let base_url = std::env::var(BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let client = BlerifyClient::new(base_url, creds, require(PROJECT_ID_ENV));

    let roles = client.get_own_roles(None).await.expect("roles fetched");

    assert!(
        !roles.is_empty(),
        "a functioning service account should hold at least one role"
    );
    for role in &roles {
        eprintln!(
            "role: {} project_id={:?} project_name={:?}",
            role.name, role.project_id, role.project_name
        );
        assert!(!role.name.is_empty());
    }
}
