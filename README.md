# rust-mdl

Rust client library for Blerify's Issuance API. Wraps the 3-step mdoc assembly
protocol (generate → sign → assemble) for ISO 18013-5 mobile documents.

Intended for Rust-based issuer backends: hides the JWT-bearer auth handshake
and token caching behind one [`BlerifyClient`], exposes typed request/response
types matching the live wire shape, and surfaces parsed server error envelopes
via [`BlerifyError::Server`].

## Scope

- Service-account authentication (custom JWT bearer flow — **not** standard
  OAuth2 `client_credentials`) with mutex-guarded token caching.
- `generate()` — `POST /api/v1/organizations/{org}/projects/{project}/credentials`,
  ISO 18013-5 §7.2.1 Table 5 mDL data elements (mandatory fields enforced by
  the type system).
- `assemble()` — `PUT .../credentials/{cid}/sign` to combine the unsigned
  credential with an ES256 signature produced externally (HSM/KMS) and return
  the final hex-encoded CBOR mdoc.

Not in scope: server-side test signing (`/crypto/sign/es256`), credential
hold/revoke/validate, wallet-side presentation, W3C VC issuance, ISO 23220.

## Quick start

```rust
use rust_mdl::{
    AssembleRequest, BlerifyClient, ServiceAccountCredentials,
    AdditionalData, DrivingCode, DrivingPrivilege, GenerateRequest,
    JwkP256, MdlData, Options, OrganizationUser, ValidityInfo,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = ServiceAccountCredentials::from_file("config/credentials.json")?;
    let client = BlerifyClient::new(
        "https://api.demo.blerify.com",
        creds,
        "057bd751-5bf6-4f98-9b75-9ec284150709", // project id
    );

    // 1. Generate the unsigned credential.
    let mut mdl_data = MdlData::new(
        "Doe", "John",                         // family_name, given_name
        "1987-03-15", "2025-10-15",            // birth_date, issue_date
        "2028-09-30", "US",                    // expiry_date, issuing_country
        "Acme", "8-203-1365",                  // issuing_authority, document_number
        "FFD8FFE000…",                          // portrait (hex JPEG bytes)
        vec![DrivingPrivilege {
            vehicle_category_code: "C".into(),
            issue_date: "2025-08-25".into(),
            expiry_date: "2028-09-30".into(),
            codes: vec![DrivingCode { code: "210".into() }],
        }],
        "PA",                                   // un_distinguishing_sign
    );
    mdl_data.nationality = Some("PA".into());

    let gen = client
        .generate(
            &GenerateRequest {
                template_id: "ca214a52-2291-4ad6-9b87-3d8fe988b0cc".into(),
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
                    certificate: include_str!("../path/to/issuer-cert.pem").into(),
                    kid: "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ".into(),
                    namespaces: vec![],
                },
                organization_user: OrganizationUser {
                    id: "8-203-1365".into(),
                    did: "did:lac1:…".into(),
                },
                options: Options { additional_data: true, onboard: false, update: true },
            },
            None, // correlation_id (auto-generated)
        )
        .await?;

    println!("credential id: {}", gen.credential.id);

    // 2. Sign `gen.signing_message` (standard base64-encoded ToBeSigned bytes) with
    //    your HSM/KMS using ES256. The result must be 128 lowercase hex chars
    //    (raw r || s, PLAIN format). This step is the caller's responsibility
    //    — rust-mdl does not handle private keys.
    let signature_hex: String = sign_with_hsm(&gen.signing_message)?;

    // 3. Assemble the final mdoc.
    let asm = client
        .assemble(
            &gen.credential.id,
            &AssembleRequest {
                template_id: "ca214a52-2291-4ad6-9b87-3d8fe988b0cc".into(),
                signature: signature_hex,
                kid: "gpWQnAjvAdLWCqQAFNglAVHlqVajGmZTPQ".into(),
                certificate: include_str!("../path/to/issuer-cert.pem").into(),
            },
            None,
        )
        .await?;

    let mdoc_bytes = hex::decode(&asm.mdoc)?; // optional — raw CBOR
    println!("mdoc: {} bytes", mdoc_bytes.len());
    Ok(())
}

# fn sign_with_hsm(_msg: &str) -> Result<String, Box<dyn std::error::Error>> { unimplemented!() }
```

## Configuration

| Concern | Default | Override |
|---|---|---|
| Connect timeout | 5 s | `BlerifyClient::from_parts` with custom `reqwest::Client` |
| Request timeout | 30 s | same |
| Token TTL | server-supplied `expires_in`, or 3600 s | n/a |
| Token refresh skew | 60 s before expiry | n/a (compile-time constant) |

Tracing spans are emitted via the `tracing` crate on every `generate`,
`assemble`, and token-mint call. Wire up `tracing-subscriber` in your binary
to capture them.

## Live integration tests

Unit tests run on every PR (`cargo test`). The two integration tests under
`tests/` are gated on `BLERIFY_RUN_LIVE_TESTS=1` so CI doesn't burn API
quota; run locally to validate against a real environment:

```bash
BLERIFY_RUN_LIVE_TESTS=1 \
BLERIFY_CREDS_PATH=/path/to/credentials.json \
BLERIFY_PROJECT_ID=<project-uuid> \
BLERIFY_TEMPLATE_ID=<template-uuid> \
BLERIFY_BASE_URL=https://api.demo.blerify.com  # optional
cargo test --tests -- --nocapture
```

## Contributing

Branch names: `feature/`, `bugfix/`, `hotfix/`, `release/`, `chore/`, `test/`, `experiment/`.

Commit messages follow Conventional Commits: `type(scope): lowercase description`
where `type ∈ { feat, fix, chore, docs, style, refactor, test, build, perf, revert }`.

All commits must be signed (GPG, SSH, or S/MIME) and show as **Verified** on
GitHub. See [GitHub's signing docs](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification).

Branch and commit conventions, plus `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test`, are enforced by
`.github/workflows/pr-validations.yml` on every PR.
