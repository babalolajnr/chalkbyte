//! User data models and DTOs.
//!
//! This module re-exports user models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all user models from the shared crate
pub use chalkbyte_models::users::*;
