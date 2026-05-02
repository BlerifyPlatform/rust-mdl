//! Live integration test against a real Blerify Issuance API instance.
//!
//! Disabled by default. Run with:
//!
//! ```bash
//! BLERIFY_RUN_LIVE_TESTS=1 \
//! BLERIFY_CREDS_PATH=/path/to/credentials.json \
//! BLERIFY_PROJECT_ID=<project-uuid> \
//! BLERIFY_TEMPLATE_ID=<template-uuid> \
//! BLERIFY_BASE_URL=https://api.demo.blerify.com  # optional, default
//! cargo test --test live_generate -- --nocapture
//! ```
//!
//! The test issues a real (test) credential against the configured staging
//! project. CI does not set `BLERIFY_RUN_LIVE_TESTS` so the test is skipped.

use rust_mdl::generate::{
    AdditionalData, DrivingCode, DrivingPrivilege, GenerateRequest, JwkP256, MdlData, Options,
    OrganizationUser, ValidityInfo,
};
use rust_mdl::{BlerifyClient, ServiceAccountCredentials};

const FLAG_ENV: &str = "BLERIFY_RUN_LIVE_TESTS";
const CREDS_PATH_ENV: &str = "BLERIFY_CREDS_PATH";
const PROJECT_ID_ENV: &str = "BLERIFY_PROJECT_ID";
const TEMPLATE_ID_ENV: &str = "BLERIFY_TEMPLATE_ID";
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
async fn on_hold_round_trip_against_staging() {
    if skip_unless_live().is_none() {
        return;
    }

    let creds =
        ServiceAccountCredentials::from_file(require(CREDS_PATH_ENV)).expect("load credentials");
    let project_id = require(PROJECT_ID_ENV);
    let template_id = require(TEMPLATE_ID_ENV);
    let base_url = std::env::var(BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into());

    let client = BlerifyClient::new(base_url, creds, project_id);

    let mut mdl_data = MdlData::new(
        "Maravi",
        "Washington",
        "1987-03-15",
        "2025-10-15",
        "2028-09-30",
        "US",
        "Acme",
        "8-203-1365",
        // Hex-encoded JPEG. Use a minimal placeholder for the integration test;
        // a real issuer would resize and JPEG-encode the citizen photo per
        // ISO 18013-5 §7.2.2 (FaceTec-grade JPEG, ≤30 KB target — see RNPN cronograma).
        "FFD8FFE000",
        vec![DrivingPrivilege {
            vehicle_category_code: "C".into(),
            issue_date: "2025-08-25".into(),
            expiry_date: "2028-09-30".into(),
            codes: vec![DrivingCode { code: "210".into() }],
        }],
        "PA",
    );
    mdl_data.nationality = Some("PA".into());

    let request = GenerateRequest {
        template_id,
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
            certificate: include_str!("fixtures/issuer-cert.pem").to_string(),
            kid: "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ".into(),
            namespaces: vec![],
        },
        organization_user: OrganizationUser {
            id: "8-203-1365".into(),
            did: "did:lac1:1iT5g9gduT4Q5DWE2bnncfnBCnM9uXPWMrCTvhPf2a8wpHWJgFBEZn295t1h9ucnQyvJ"
                .into(),
        },
        options: Options {
            additional_data: true,
            onboard: false,
            update: true,
        },
    };

    let response = client
        .generate(&request, None)
        .await
        .expect("generate succeeds");

    let on_hold_response = client
        .on_hold(&response.credential.id, None)
        .await
        .expect("on_hold succeeds");

    assert!(
        !on_hold_response.status.is_empty(),
        "status must not be empty"
    );

    eprintln!("onHold status: {}", on_hold_response.status);

    assert!(
        response.credential.id.starts_with("0x"),
        "credential id should be 0x-prefixed, got {}",
        response.credential.id,
    );
    assert_eq!(
        response.credential.id.len(),
        66,
        "expected 0x + 64 hex chars"
    );
    assert!(
        !response.signing_message.is_empty(),
        "signingMessage must not be empty"
    );

    eprintln!("credential id: {}", response.credential.id);
    eprintln!(
        "signing message length: {} chars",
        response.signing_message.len()
    );
}
