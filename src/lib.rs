//! Rust client for Blerify's Issuance API.
//!
//! Provides service-account authentication, a token cache, and an HTTP client
//! scoped to a single `(organization, project)` pair. Endpoint methods:
//!
//! - [`BlerifyClient::generate`] — `POST /credentials`
//! - [`BlerifyClient::assemble`] — `PUT /credentials/{id}/sign`
//! - [`BlerifyClient::revoke`]   — `PUT /credentials/{id}/revoke`

pub mod assemble;
pub mod auth;
pub mod client;
pub mod credentials;
pub mod error;
pub mod generate;
pub mod revoke;
pub mod on_hold;
pub mod validate;

pub use assemble::{AssembleRequest, AssembleResponse};
pub use client::BlerifyClient;
pub use credentials::ServiceAccountCredentials;
pub use error::BlerifyError;
pub use generate::{
    AdditionalData, DrivingCode, DrivingPrivilege, GenerateRequest, GenerateResponse,
    GeneratedCredential, JwkP256, MdlData, NamespaceEntry, Options, OrganizationUser, TemplateInfo,
    ValidityInfo,
};
pub use revoke::{RevokeRequest, RevokeResponse, StateChangeMetadata};
pub use on_hold::OnHoldResponse;
pub use validate::ValidateResponse;