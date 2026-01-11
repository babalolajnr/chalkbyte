//! Utility modules for the Chalkbyte API.
//!
//! This module contains shared utilities used throughout the application:
//!
//! - [`auth_helpers`]: Helper functions for authentication and authorization
//! - [`email`]: Email sending utilities using SMTP
//! - [`errors`]: Application error types and handling
//! - [`jwt`]: JWT token creation and verification
//! - [`pagination`]: Request pagination utilities
//! - [`password`]: Password hashing and verification
//! - [`serde`]: Custom serde serialization/deserialization helpers
//! - [`tracing`]: Distributed tracing utilities

pub mod auth_helpers;
pub mod email;
pub mod errors;
pub mod jwt;
pub mod pagination;
pub mod password;
pub mod serde;
pub mod tracing;
