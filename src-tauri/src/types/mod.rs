//! Shared DTO types aligned with `src/lib/api/contracts.ts` (SSOT).
//!
//! These types unify a few cross-cutting shapes the frontend adapter
//! previously had to synthesize:
//! - [`OperationResult`] — replaces ad-hoc `()` / `bool` returns whose
//!   semantics carry "operation succeeded" but no payload.
//! - [`ProfileQueryResult`] — wraps `Option<Profile>` so the frontend can
//!   distinguish a successful "no active profile" result from an error.
//! - [`LocalizedString`] / [`TypedError`] — re-exported so probe / installer
//!   layers can build structured messages without duplicating the structs.

use serde::{Deserialize, Serialize};

use crate::services::profile::Profile;

pub use crate::services::profile::{LocalizedString, TypedError};

/// Mirrors `contracts.ts::OperationResult` — used as the success return for
/// fire-and-forget commands that previously returned `()` or `bool`. On
/// failure the command still returns `Err(...)`; this struct is the success
/// envelope only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<LocalizedString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl OperationResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
            error_code: None,
        }
    }
}

/// Wraps `Option<Profile>` so the frontend gets `{ profile: null }`
/// instead of a bare `null` (which is harder to type-check in zod).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileQueryResult {
    pub profile: Option<Profile>,
}

impl ProfileQueryResult {
    pub fn new(profile: Option<Profile>) -> Self {
        Self { profile }
    }
}
