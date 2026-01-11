//! Utility modules for the Chalkbyte API.
#![allow(unused_imports)]
//!
//! This module contains shared utilities used throughout the application.
//! Core utilities have been moved to the `chalkbyte-core` crate for better
//! compilation performance. This module re-exports them for convenience.
//!
//! ## Re-exported from `chalkbyte-core`
//!
//! - [`errors`]: Application error types and handling
//! - [`pagination`]: Request pagination utilities
//! - [`password`]: Password hashing and verification
//! - [`serde`]: Custom serde serialization/deserialization helpers
//!
//! ## Local modules
//!
//! - [`auth_helpers`]: Helper functions for authentication and authorization
//! - [`email`]: Email sending utilities using SMTP
//! - [`jwt`]: JWT token creation and verification (re-exports from `chalkbyte-auth`)
//! - [`tracing`]: Distributed tracing utilities

// Re-export from chalkbyte-core
pub use chalkbyte_core::errors;
pub use chalkbyte_core::pagination;
pub use chalkbyte_core::password;
pub use chalkbyte_core::serde;

// Re-export from chalkbyte-auth
pub mod jwt {
    //! JWT utilities re-exported from `chalkbyte-auth`.
    pub use chalkbyte_auth::*;
}

// Local modules
pub mod auth_helpers;
pub mod email;
pub mod tracing;
