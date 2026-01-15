//! Academic session data models and DTOs.
//!
//! This module re-exports academic session models from the `chalkbyte-models` crate
//! for backward compatibility and provides any controller-specific types.

// Re-export all academic session models from the shared crate
pub use chalkbyte_models::academic_sessions::*;
