//! Application state management.
//!
//! This module defines the shared application state that is passed to all
//! Axum handlers. It contains database connections, configuration, and
//! other shared resources.
//!
//! # Example
//!
//! ```ignore
//! use crate::state::{AppState, init_app_state};
//!
//! #[tokio::main]
//! async fn main() {
//!     let state = init_app_state().await;
//!     // Pass state to router
//!     let app = Router::new().with_state(state);
//! }
//! ```

use std::time::Duration;

use chalkbyte_cache::{CacheConfig, RedisCache};
use chalkbyte_config::{CorsConfig, EmailConfig, JwtConfig, RateLimitConfig};
use chalkbyte_db::{PgPool, init_db_pool};
use tracing::{info, warn};

/// Shared application state passed to all request handlers.
///
/// This struct is cloned for each request, so all fields must be cheaply
/// cloneable (e.g., using `Arc` internally or being `Copy`).
///
/// # Fields
///
/// - `db`: PostgreSQL connection pool for database operations
/// - `jwt_config`: JWT configuration for token creation/verification
/// - `email_config`: Email/SMTP configuration for sending emails
/// - `cors_config`: CORS configuration for cross-origin requests
/// - `rate_limit_config`: Rate limiting configuration (reserved for future use)
/// - `cache`: Optional Redis cache for distributed caching
#[derive(Clone, Debug)]
pub struct AppState {
    /// PostgreSQL connection pool.
    ///
    /// Use this pool to execute database queries in handlers and services.
    pub db: PgPool,

    /// JWT configuration for authentication.
    ///
    /// Contains the secret key and token expiry settings.
    pub jwt_config: JwtConfig,

    /// Email configuration for SMTP.
    ///
    /// Used for sending password reset emails and notifications.
    #[allow(dead_code)]
    pub email_config: EmailConfig,

    /// CORS configuration.
    ///
    /// Defines allowed origins, methods, and headers for cross-origin requests.
    pub cors_config: CorsConfig,

    /// Rate limiting configuration.
    ///
    /// Defines rate limits for API endpoints (reserved for future use).
    #[allow(dead_code)]
    pub rate_limit_config: RateLimitConfig,

    /// Redis cache configuration.
    ///
    /// Used for cache key generation and TTL settings.
    pub cache_config: CacheConfig,

    /// Redis cache client for distributed caching.
    ///
    /// Optional - if Redis is unavailable, the application continues without caching.
    pub cache: Option<RedisCache>,
}

/// Initializes the application state with all required configurations.
///
/// This function:
/// 1. Initializes the database connection pool
/// 2. Loads JWT configuration from environment variables
/// 3. Loads email configuration from environment variables
/// 4. Loads CORS configuration from environment variables
/// 5. Loads rate limit configuration from environment variables
/// 6. Initializes Redis cache (optional, continues without if unavailable)
///
/// # Panics
///
/// Panics if the database connection cannot be established.
///
/// # Example
///
/// ```ignore
/// let state = init_app_state().await;
/// ```
pub async fn init_app_state() -> AppState {
    let cache_config = CacheConfig::from_env();
    let cache = init_cache(&cache_config).await;

    AppState {
        db: init_db_pool().await,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
        rate_limit_config: RateLimitConfig::from_env(),
        cache_config,
        cache,
    }
}

/// Initializes the Redis cache client.
///
/// Returns `None` if Redis connection fails, allowing the application
/// to continue without caching.
async fn init_cache(config: &CacheConfig) -> Option<RedisCache> {
    match RedisCache::new(
        &config.redis_url,
        Duration::from_secs(config.default_ttl_seconds),
    )
    .await
    {
        Ok(cache) => {
            info!(redis_url = %config.redis_url, "Redis cache initialized");
            Some(cache)
        }
        Err(e) => {
            warn!(
                error = %e,
                redis_url = %config.redis_url,
                "Failed to connect to Redis, continuing without cache"
            );
            None
        }
    }
}
