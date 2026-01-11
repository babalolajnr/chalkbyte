//! Student data models and DTOs.
//!
//! This module re-exports student models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all student models from the shared crate
pub use chalkbyte_models::students::*;
