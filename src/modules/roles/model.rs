//! Role and permission data models and DTOs.
//!
//! This module re-exports role models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all role models from the shared crate
pub use chalkbyte_models::roles::*;
