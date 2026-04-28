//! Rust client for Blerify's Issuance API.
//!
//! Provides service-account authentication, a token cache, and an HTTP client
//! scoped to a single `(organization, project)` pair. Endpoint methods land
//! in upcoming slices:
//!
//! - `generate()`  — POST `/credentials` (this slice)
//! - `assemble()`  — PUT `/credentials/{id}/sign` (slice 1.4)

pub mod auth;
pub mod client;
pub mod credentials;
pub mod error;
pub mod generate;

pub use client::BlerifyClient;
pub use credentials::ServiceAccountCredentials;
pub use error::BlerifyError;
pub use generate::{
    AdditionalData, DrivingCode, DrivingPrivilege, GenerateRequest, GenerateResponse,
    GeneratedCredential, JwkP256, MdlData, NamespaceEntry, Options, OrganizationUser, TemplateInfo,
    ValidityInfo,
};
