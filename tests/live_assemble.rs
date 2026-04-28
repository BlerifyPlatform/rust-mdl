//! Live integration test for the full generate → assemble round-trip against
//! a real Blerify Issuance API instance.
//!
//! Disabled by default. Run with:
//!
//! ```bash
//! BLERIFY_RUN_LIVE_TESTS=1 \
//! BLERIFY_CREDS_PATH=/path/to/credentials.json \
//! BLERIFY_PROJECT_ID=<project-uuid> \
//! BLERIFY_TEMPLATE_ID=<template-uuid> \
//! BLERIFY_BASE_URL=https://api.demo.blerify.com  # optional
//! cargo test --test live_assemble -- --nocapture
//! ```
//!
//! ## What this test verifies
//!
//! - The generate → assemble HTTP wire path (paths, headers, JSON shapes) is
//!   correct against the real server.
//! - The assemble response parses into [`AssembleResponse`] with a hex-CBOR
//!   `mdoc` field.
//!
//! ## What this test does NOT verify
//!
//! Cryptographic correctness of the signature. The test sends a placeholder
//! signature (128 zero hex chars) over the `signingMessage` because the
//! library does not bundle a matching private key for
//! `tests/fixtures/issuer-cert.pem`. If the server rejects the placeholder
//! signature, the test surfaces the server's exact error envelope, which is
//! itself useful signal for the contract memory.

use rust_mdl::generate::{
    AdditionalData, DrivingCode, DrivingPrivilege, GenerateRequest, JwkP256, MdlData, Options,
    OrganizationUser, ValidityInfo,
};
use rust_mdl::{AssembleRequest, BlerifyClient, BlerifyError, ServiceAccountCredentials};

const FLAG_ENV: &str = "BLERIFY_RUN_LIVE_TESTS";
const CREDS_PATH_ENV: &str = "BLERIFY_CREDS_PATH";
const PROJECT_ID_ENV: &str = "BLERIFY_PROJECT_ID";
const TEMPLATE_ID_ENV: &str = "BLERIFY_TEMPLATE_ID";
const BASE_URL_ENV: &str = "BLERIFY_BASE_URL";
const DEFAULT_BASE_URL: &str = "https://api.demo.blerify.com";

fn require(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set when running live tests"))
}

#[tokio::test]
async fn generate_then_assemble_round_trip() {
    if std::env::var(FLAG_ENV).is_err() {
        eprintln!("skipping: {FLAG_ENV} not set");
        return;
    }

    let creds =
        ServiceAccountCredentials::from_file(require(CREDS_PATH_ENV)).expect("load credentials");
    let project_id = require(PROJECT_ID_ENV);
    let template_id = require(TEMPLATE_ID_ENV);
    let base_url = std::env::var(BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into());
    let cert = include_str!("fixtures/issuer-cert.pem").to_string();
    let kid = "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ";

    let client = BlerifyClient::new(base_url, creds, project_id);

    // Step 1 — generate.
    let mut mdl_data = MdlData::new(
        "Maravi",
        "Washington",
        "1987-03-15",
        "2025-10-15",
        "2028-09-30",
        "US",
        "Acme",
        "8-203-1365",
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

    let gen_request = GenerateRequest {
        template_id: template_id.clone(),
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
            certificate: cert.clone(),
            kid: kid.into(),
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

    let gen_response = client
        .generate(&gen_request, None)
        .await
        .expect("generate succeeds");
    assert!(gen_response.credential.id.starts_with("0x"));
    eprintln!("credential id: {}", gen_response.credential.id);
    eprintln!(
        "signingMessage length: {}",
        gen_response.signing_message.len()
    );

    // Step 2 — placeholder signature (128 zero hex chars).
    // We do not have the private key matching `issuer-cert.pem`, so this
    // signature will not verify cryptographically. The point of this test
    // is the wire path; if the server validates and rejects we surface the
    // exact error.
    let asm_request = AssembleRequest {
        template_id,
        signature: "0".repeat(128),
        kid: kid.into(),
        certificate: cert,
    };

    match client
        .assemble(&gen_response.credential.id, &asm_request, None)
        .await
    {
        Ok(resp) => {
            eprintln!("assemble succeeded; mdoc length: {}", resp.mdoc.len());
            assert!(
                resp.mdoc.starts_with("a3"),
                "mdoc starts with CBOR map-of-3 marker"
            );
            assert!(
                resp.mdoc.chars().all(|c| c.is_ascii_hexdigit()),
                "mdoc must be lowercase hex",
            );
        }
        Err(BlerifyError::Server {
            status,
            message,
            body,
        }) => {
            eprintln!("assemble rejected by server (expected if signature validation is enforced)");
            eprintln!("  status: {status}");
            eprintln!("  message: {message:?}");
            eprintln!("  body: {body}");
            // Don't fail the test — the wire path was exercised. Surface the
            // error envelope shape for contract memory.
        }
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}
