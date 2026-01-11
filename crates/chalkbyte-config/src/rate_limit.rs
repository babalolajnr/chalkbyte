//! Rate limiting configuration for API endpoints.
//!
//! This module provides configuration for rate limiting using the Governor crate.
//! Rate limits help protect the API from abuse and ensure fair usage.
//!
//! # Configuration
//!
//! Rate limits can be configured via environment variables:
//!
//! - `RATE_LIMIT_GENERAL_PER_SECOND`: Requests per second for general endpoints (default: 2)
//! - `RATE_LIMIT_GENERAL_BURST_SIZE`: Burst size for general endpoints (default: 30)
//! - `RATE_LIMIT_AUTH_PER_SECOND`: Requests per second for auth endpoints (default: 10)
//! - `RATE_LIMIT_AUTH_BURST_SIZE`: Burst size for auth endpoints (default: 5)
//!
//! # Rate Limiting Strategy
//!
//! The rate limiter uses a token bucket algorithm:
//!
//! - Tokens are added at the configured rate (per second)
//! - Each request consumes one token
//! - Burst size defines the maximum tokens that can accumulate
//! - Requests are rejected when no tokens are available
//!
//! # Example
//!
//! ```ignore
//! use crate::config::rate_limit::RateLimitConfig;
//!
//! let config = RateLimitConfig::from_env();
//!
//! // Create governor config for general endpoints
//! let governor = config.general_governor_config();
//! ```

use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::PeerIpKeyExtractor;

/// Rate limit configuration for the API.
///
/// Defines separate rate limits for general API endpoints and authentication
/// endpoints (which typically need stricter limits to prevent brute-force attacks).
///
/// # Fields
///
/// - `general_per_second`: Token replenishment rate for general endpoints
/// - `general_burst_size`: Maximum token accumulation for general endpoints
/// - `auth_per_second`: Token replenishment rate for auth endpoints
/// - `auth_burst_size`: Maximum token accumulation for auth endpoints
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Requests per second for general endpoints.
    ///
    /// This is the rate at which tokens are replenished in the bucket.
    #[allow(dead_code)]
    pub general_per_second: u64,

    /// Burst size for general endpoints.
    ///
    /// This is the maximum number of tokens that can accumulate,
    /// allowing short bursts of traffic above the per-second rate.
    #[allow(dead_code)]
    pub general_burst_size: u32,

    /// Requests per second for auth endpoints (stricter).
    ///
    /// Auth endpoints have stricter limits to prevent brute-force attacks
    /// on login and password reset endpoints.
    #[allow(dead_code)]
    pub auth_per_second: u64,

    /// Burst size for auth endpoints (stricter).
    ///
    /// Lower burst size for auth endpoints provides additional protection
    /// against rapid-fire authentication attempts.
    #[allow(dead_code)]
    pub auth_burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            general_per_second: 2,
            general_burst_size: 30,
            auth_per_second: 10,
            auth_burst_size: 5,
        }
    }
}

impl RateLimitConfig {
    /// Creates a new `RateLimitConfig` from environment variables.
    ///
    /// Falls back to default values if environment variables are not set
    /// or cannot be parsed.
    ///
    /// # Environment Variables
    ///
    /// - `RATE_LIMIT_GENERAL_PER_SECOND`: Default 2
    /// - `RATE_LIMIT_GENERAL_BURST_SIZE`: Default 30
    /// - `RATE_LIMIT_AUTH_PER_SECOND`: Default 10
    /// - `RATE_LIMIT_AUTH_BURST_SIZE`: Default 5
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            general_per_second: std::env::var("RATE_LIMIT_GENERAL_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            general_burst_size: std::env::var("RATE_LIMIT_GENERAL_BURST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            auth_per_second: std::env::var("RATE_LIMIT_AUTH_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            auth_burst_size: std::env::var("RATE_LIMIT_AUTH_BURST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }

    /// Creates a `GovernorConfig` for general API endpoints.
    ///
    /// The returned config uses the peer IP address as the rate limit key,
    /// meaning each IP address has its own rate limit bucket.
    ///
    /// # Panics
    ///
    /// Panics if the governor configuration cannot be built (should not happen
    /// with valid configuration values).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = RateLimitConfig::from_env();
    /// let governor = config.general_governor_config();
    ///
    /// // Use with tower_governor middleware
    /// let app = Router::new()
    ///     .layer(GovernorLayer { config: governor });
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn general_governor_config(
        &self,
    ) -> GovernorConfig<PeerIpKeyExtractor, ::governor::middleware::NoOpMiddleware> {
        GovernorConfigBuilder::default()
            .per_second(self.general_per_second)
            .burst_size(self.general_burst_size)
            .key_extractor(PeerIpKeyExtractor)
            .finish()
            .expect("Failed to build general rate limiter config")
    }

    /// Creates a `GovernorConfig` for authentication endpoints.
    ///
    /// Auth endpoints have stricter rate limits to prevent brute-force attacks.
    /// The returned config uses the peer IP address as the rate limit key.
    ///
    /// # Panics
    ///
    /// Panics if the governor configuration cannot be built.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = RateLimitConfig::from_env();
    /// let auth_governor = config.auth_governor_config();
    ///
    /// // Apply stricter limits to auth routes
    /// let auth_router = Router::new()
    ///     .route("/login", post(login))
    ///     .layer(GovernorLayer { config: auth_governor });
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn auth_governor_config(
        &self,
    ) -> GovernorConfig<PeerIpKeyExtractor, ::governor::middleware::NoOpMiddleware> {
        GovernorConfigBuilder::default()
            .per_second(self.auth_per_second)
            .burst_size(self.auth_burst_size)
            .key_extractor(PeerIpKeyExtractor)
            .finish()
            .expect("Failed to build auth rate limiter config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.general_per_second, 2);
        assert_eq!(config.general_burst_size, 30);
        assert_eq!(config.auth_per_second, 10);
        assert_eq!(config.auth_burst_size, 5);
    }

    #[test]
    fn test_config_equality() {
        let config1 = RateLimitConfig::default();
        let config2 = RateLimitConfig::default();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_config_clone() {
        let config = RateLimitConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_config_debug() {
        let config = RateLimitConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("RateLimitConfig"));
        assert!(debug_str.contains("general_per_second"));
    }
}
