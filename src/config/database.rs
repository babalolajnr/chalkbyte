//! Database configuration and connection pool initialization.
//!
//! This module handles PostgreSQL database connection pool setup using SQLx.
//! The database URL is read from the `DATABASE_URL` environment variable.
//!
//! # Environment Variables
//!
//! - `DATABASE_URL`: PostgreSQL connection string (required)
//!
//! # Connection String Format
//!
//! ```text
//! postgres://username:password@host:port/database_name
//! ```
//!
//! # Example
//!
//! ```ignore
//! use crate::config::database::init_db_pool;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Ensure DATABASE_URL is set
//!     std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/chalkbyte");
//!
//!     let pool = init_db_pool().await;
//!     // Use pool for database operations
//! }
//! ```
//!
//! # Connection Pool
//!
//! SQLx manages a pool of database connections automatically. The pool:
//!
//! - Reuses connections to reduce overhead
//! - Handles connection failures and reconnection
//! - Provides async/await support for non-blocking queries
//!
//! # Panics
//!
//! The [`init_db_pool`] function will panic if:
//!
//! - `DATABASE_URL` environment variable is not set
//! - The database connection cannot be established

use sqlx::PgPool;
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
pub async fn init_db_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}
