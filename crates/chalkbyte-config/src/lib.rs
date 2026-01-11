//! # Chalkbyte Config
//!
//! Configuration types for the Chalkbyte API.
//!
//! This crate provides configuration structures loaded from environment variables:
//!
//! - [`jwt`]: JWT authentication configuration
//! - [`cors`]: CORS (Cross-Origin Resource Sharing) configuration
//! - [`email`]: Email/SMTP configuration
//! - [`rate_limit`]: API rate limiting configuration
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_config::{JwtConfig, CorsConfig, EmailConfig, RateLimitConfig};
//!
//! // Load all configs from environment
//! let jwt_config = JwtConfig::from_env();
//! let cors_config = CorsConfig::from_env();
//! let email_config = EmailConfig::from_env();
//! let rate_limit_config = RateLimitConfig::from_env();
//! ```

pub mod cors;
pub mod email;
pub mod jwt;
pub mod rate_limit;

// Re-export commonly used types at crate root
pub use cors::CorsConfig;
pub use email::EmailConfig;
pub use jwt::JwtConfig;
pub use rate_limit::RateLimitConfig;
