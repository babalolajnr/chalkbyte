//! Redis cache configuration.
//!
//! This module provides configuration for Redis connection settings
//! loaded from environment variables.

use std::env;

/// Redis cache configuration loaded from environment variables.
///
/// # Environment Variables
///
/// - `REDIS_URL`: Redis connection URL (default: `redis://127.0.0.1:6379`)
/// - `CACHE_TTL_SECONDS`: Default TTL for cached items in seconds (default: `300`)
/// - `CACHE_PREFIX`: Prefix for all cache keys (default: `chalkbyte`)
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Redis connection URL.
    pub redis_url: String,

    /// Default time-to-live for cached items in seconds.
    pub default_ttl_seconds: u64,

    /// Prefix for all cache keys to avoid collisions.
    pub key_prefix: String,
}

impl CacheConfig {
    /// Load configuration from environment variables.
    ///
    /// # Defaults
    ///
    /// - `REDIS_URL`: `redis://127.0.0.1:6379`
    /// - `CACHE_TTL_SECONDS`: `300` (5 minutes)
    /// - `CACHE_PREFIX`: `chalkbyte`
    pub fn from_env() -> Self {
        Self {
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            default_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            key_prefix: env::var("CACHE_PREFIX").unwrap_or_else(|_| "chalkbyte".into()),
        }
    }

    /// Build a prefixed cache key.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = CacheConfig::from_env();
    /// let key = config.prefixed_key("school:123");
    /// // Returns "chalkbyte:school:123"
    /// ```
    pub fn prefixed_key(&self, key: &str) -> String {
        format!("{}:{}", self.key_prefix, key)
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".into(),
            default_ttl_seconds: 300,
            key_prefix: "chalkbyte".into(),
        }
    }
}
