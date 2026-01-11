//! JWT (JSON Web Token) configuration.
//!
//! This module provides configuration for JWT-based authentication,
//! including secret key management and token expiry settings.
//!
//! # Environment Variables
//!
//! - `JWT_SECRET`: Secret key for signing tokens (required in production)
//! - `JWT_ACCESS_EXPIRY`: Access token expiry in seconds (default: 3600 = 1 hour)
//! - `JWT_REFRESH_EXPIRY`: Refresh token expiry in seconds (default: 604800 = 7 days)
//!
//! # Security Considerations
//!
//! - **Always set `JWT_SECRET` in production** - the default value is insecure
//! - Use a cryptographically random string of at least 32 characters
//! - Keep the secret key confidential and rotate it periodically
//! - Access tokens should be short-lived; refresh tokens can be longer
//!
//! # Example
//!
//! ```ignore
//! use crate::config::jwt::JwtConfig;
//!
//! // Load configuration from environment
//! let config = JwtConfig::from_env();
//!
//! // Use in token creation
//! let token = create_access_token(user_id, email, school_id, roles, perms, &config)?;
//! ```
//!
//! # Token Lifetimes
//!
//! The default token lifetimes are:
//!
//! | Token Type     | Default Expiry | Purpose                              |
//! |----------------|----------------|--------------------------------------|
//! | Access Token   | 1 hour         | Short-lived API authentication       |
//! | Refresh Token  | 7 days         | Obtain new access tokens             |
//! | MFA Temp Token | 10 minutes     | Complete MFA verification (hardcoded)|

use std::env;

/// JWT configuration containing secret key and token expiry settings.
///
/// This struct holds all JWT-related configuration values loaded from
/// environment variables. It is cloneable and can be shared across
/// request handlers via the application state.
///
/// # Fields
///
/// - `secret`: The secret key used for signing and verifying JWTs
/// - `access_token_expiry`: How long access tokens remain valid (in seconds)
/// - `refresh_token_expiry`: How long refresh tokens remain valid (in seconds)
///
/// # Example
///
/// ```ignore
/// let config = JwtConfig::from_env();
///
/// assert!(config.access_token_expiry > 0);
/// assert!(config.refresh_token_expiry > config.access_token_expiry);
/// ```
#[derive(Clone, Debug)]
pub struct JwtConfig {
    /// Secret key for signing and verifying JWT tokens.
    ///
    /// This should be a long, random string that is kept confidential.
    /// In production, always set this via the `JWT_SECRET` environment variable.
    ///
    /// # Security Warning
    ///
    /// The default value is insecure and should never be used in production.
    pub secret: String,

    /// Access token expiry time in seconds.
    ///
    /// Access tokens are used for API authentication. They should be
    /// short-lived to minimize the impact of token theft.
    ///
    /// Default: 3600 (1 hour)
    pub access_token_expiry: i64,

    /// Refresh token expiry time in seconds.
    ///
    /// Refresh tokens are used to obtain new access tokens without
    /// requiring the user to re-authenticate. They can be longer-lived
    /// but should still expire eventually.
    ///
    /// Default: 604800 (7 days)
    pub refresh_token_expiry: i64,
}

impl JwtConfig {
    /// Creates a new `JwtConfig` from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `JWT_SECRET`: Secret key (default: "your-secret-key-change-in-production")
    /// - `JWT_ACCESS_EXPIRY`: Access token expiry in seconds (default: 3600)
    /// - `JWT_REFRESH_EXPIRY`: Refresh token expiry in seconds (default: 604800)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set environment variables
    /// std::env::set_var("JWT_SECRET", "my-super-secret-key-at-least-32-chars");
    /// std::env::set_var("JWT_ACCESS_EXPIRY", "1800"); // 30 minutes
    ///
    /// let config = JwtConfig::from_env();
    /// assert_eq!(config.access_token_expiry, 1800);
    /// ```
    ///
    /// # Security Note
    ///
    /// If `JWT_SECRET` is not set, a warning should be logged in production
    /// environments as the default value is insecure.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            access_token_expiry: env::var("JWT_ACCESS_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600), // 1 hour
            refresh_token_expiry: env::var("JWT_REFRESH_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(604800), // 7 days
        }
    }
}

impl Default for JwtConfig {
    /// Returns the default JWT configuration.
    ///
    /// This is equivalent to calling `JwtConfig::from_env()` with no
    /// environment variables set.
    ///
    /// # Warning
    ///
    /// The default secret key is insecure. Always set `JWT_SECRET` in production.
    fn default() -> Self {
        Self {
            secret: "your-secret-key-change-in-production".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = JwtConfig::default();
        assert_eq!(config.access_token_expiry, 3600);
        assert_eq!(config.refresh_token_expiry, 604800);
        assert!(!config.secret.is_empty());
    }

    #[test]
    fn test_refresh_longer_than_access() {
        let config = JwtConfig::default();
        assert!(config.refresh_token_expiry > config.access_token_expiry);
    }

    #[test]
    fn test_config_clone() {
        let config = JwtConfig::default();
        let cloned = config.clone();
        assert_eq!(config.secret, cloned.secret);
        assert_eq!(config.access_token_expiry, cloned.access_token_expiry);
        assert_eq!(config.refresh_token_expiry, cloned.refresh_token_expiry);
    }

    #[test]
    fn test_config_debug() {
        let config = JwtConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("JwtConfig"));
        assert!(debug_str.contains("access_token_expiry"));
    }
}
