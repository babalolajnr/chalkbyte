//! Configuration modules for the Chalkbyte API.
#![allow(unused_imports)]
//!
//! This module re-exports configuration types from the `chalkbyte-config` crate
//! for convenience. The database pool initialization is re-exported from
//! `chalkbyte-db`.
//!
//! # Re-exported from `chalkbyte-config`
//!
//! - [`cors`]: CORS (Cross-Origin Resource Sharing) configuration
//! - [`email`]: Email/SMTP configuration for sending notifications
//! - [`jwt`]: JWT authentication configuration
//! - [`rate_limit`]: API rate limiting configuration
//!
//! # Re-exported from `chalkbyte-db`
//!
//! - [`database`]: PostgreSQL database connection pool initialization
//!
//! # Example
//!
//! ```ignore
//! use crate::config::jwt::JwtConfig;
//! use crate::config::database::init_db_pool;
//!
//! // Load JWT config from environment
//! let jwt_config = JwtConfig::from_env();
//!
//! // Initialize database pool
//! let db = init_db_pool().await;
//! ```

// Re-export from chalkbyte-config
pub use chalkbyte_config::cors;
pub use chalkbyte_config::email;
pub use chalkbyte_config::jwt;
pub use chalkbyte_config::rate_limit;

// Re-export database from chalkbyte-db
pub mod database {
    //! Database pool initialization re-exported from `chalkbyte-db`.
    pub use chalkbyte_db::init_db_pool;
}
