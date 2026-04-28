//! Rust client for Blerify's Issuance API.
//!
//! Provides service-account authentication, a token cache, and an HTTP client
//! scoped to a single `(organization, project)` pair. Endpoint methods land
//! in upcoming slices:
//!
//! - `generate()`  — POST `/credentials` (slice 1.3)
//! - `assemble()`  — PUT `/credentials/{id}/sign` (slice 1.4)

pub mod auth;
pub mod client;
pub mod credentials;
pub mod error;

pub use client::BlerifyClient;
pub use credentials::ServiceAccountCredentials;
pub use error::BlerifyError;
