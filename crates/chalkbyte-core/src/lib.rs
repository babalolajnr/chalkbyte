//! # Chalkbyte Core
//!
//! Core types, errors, and utilities for the Chalkbyte API.
//!
//! This crate provides foundational types used throughout the Chalkbyte application:
//!
//! - [`errors`]: Application error types with HTTP response conversion
//! - [`pagination`]: Pagination utilities for API responses
//! - [`password`]: Secure password hashing and verification
//! - [`serde`]: Custom serde serialization/deserialization helpers
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_core::errors::AppError;
//! use chalkbyte_core::pagination::{PaginationParams, PaginationMeta};
//! use chalkbyte_core::password::{hash_password, verify_password};
//!
//! // Create an error
//! let error = AppError::not_found(anyhow::anyhow!("User not found"));
//!
//! // Hash a password
//! let hash = hash_password("secure_password")?;
//!
//! // Use pagination
//! let params = PaginationParams::default();
//! let limit = params.limit();
//! ```

pub mod errors;
pub mod pagination;
pub mod password;
pub mod serde;

// Re-export commonly used types at crate root
pub use errors::AppError;
pub use pagination::{PaginationMeta, PaginationParams};
pub use password::{hash_password, verify_password};
