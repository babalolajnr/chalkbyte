//! # Chalkbyte Auth
//!
//! Authentication types and JWT utilities for the Chalkbyte API.
//!
//! This crate provides:
//!
//! - [`claims`]: JWT claim structures for access, refresh, and MFA tokens
//! - [`jwt`]: Token creation and verification utilities
//!
//! # Token Types
//!
//! The authentication system uses three types of JWT tokens:
//!
//! - **Access Token** ([`Claims`]): Short-lived token for API authentication
//! - **Refresh Token** ([`RefreshTokenClaims`]): Long-lived token for obtaining new access tokens
//! - **MFA Temp Token** ([`MfaTempClaims`]): Temporary token for MFA verification flow
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_auth::{Claims, create_access_token, verify_token};
//! use chalkbyte_config::JwtConfig;
//!
//! let config = JwtConfig::from_env();
//!
//! // Create an access token
//! let token = create_access_token(
//!     user_id,
//!     "user@example.com",
//!     Some(school_id),
//!     vec![role_id],
//!     vec!["users:read".to_string()],
//!     &config,
//! )?;
//!
//! // Verify the token
//! let claims = verify_token(&token, &config)?;
//! println!("User ID: {}", claims.sub);
//! ```

pub mod claims;
pub mod jwt;

// Re-export commonly used types at crate root
pub use claims::{Claims, MfaTempClaims, RefreshTokenClaims};
pub use jwt::{
    create_access_token, create_mfa_temp_token, create_refresh_token, verify_mfa_temp_token,
    verify_refresh_token, verify_token,
};
