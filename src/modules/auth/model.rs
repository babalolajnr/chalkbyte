//! Authentication data models and DTOs.
//!
//! This module re-exports authentication models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all auth models from the shared crate
pub use chalkbyte_models::auth::*;
