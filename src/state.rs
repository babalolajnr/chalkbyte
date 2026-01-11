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

use chalkbyte_config::{CorsConfig, EmailConfig, JwtConfig, RateLimitConfig};
use chalkbyte_db::{PgPool, init_db_pool};

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
}

/// Initializes the application state with all required configurations.
///
/// This function:
/// 1. Initializes the database connection pool
/// 2. Loads JWT configuration from environment variables
/// 3. Loads email configuration from environment variables
/// 4. Loads CORS configuration from environment variables
/// 5. Loads rate limit configuration from environment variables
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
    AppState {
        db: init_db_pool().await,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
        rate_limit_config: RateLimitConfig::from_env(),
    }
}
