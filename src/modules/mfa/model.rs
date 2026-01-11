//! MFA (Multi-Factor Authentication) data models and DTOs.
//!
//! This module re-exports MFA models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all MFA models from the shared crate
pub use chalkbyte_models::mfa::*;
