//! # Chalkbyte DB
//!
//! Database pool and utilities for the Chalkbyte API.
//!
//! This crate provides database connection pool initialization and management
//! using SQLx with PostgreSQL.
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_db::init_db_pool;
//!
//! #[tokio::main]
//! async fn main() {
//!     let pool = init_db_pool().await;
//!     // Use pool for database operations
//! }
//! ```

use std::env;

/// Initializes a PostgreSQL connection pool.
///
/// This function reads the database URL from the `DATABASE_URL` environment
/// variable and creates a connection pool. The pool is used throughout the
/// application for all database operations.
///
/// # Returns
///
/// Returns a [`PgPool`] that can be cloned and shared across async tasks.
///
/// # Panics
///
/// Panics if:
/// - `DATABASE_URL` environment variable is not set
/// - Connection to the database fails
///
/// # Example
///
/// ```ignore
/// let pool = init_db_pool().await;
///
/// // Use the pool in queries
/// let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
///     .fetch_one(&pool)
///     .await?;
/// ```
///
/// # Note
///
/// This function should typically be called once during application startup.
/// The returned pool is cheaply cloneable and should be passed to the
/// application state for use in request handlers.
pub async fn init_db_pool() -> sqlx::PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

// Re-export PgPool for convenience
pub use sqlx::PgPool;
