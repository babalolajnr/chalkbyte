//! Configuration modules for the Chalkbyte API.
//!
//! This module contains all configuration-related types and utilities
//! for the application. Each submodule handles a specific aspect of
//! configuration, typically loaded from environment variables.
//!
//! # Modules
//!
//! - [`cors`]: CORS (Cross-Origin Resource Sharing) configuration
//! - [`database`]: PostgreSQL database connection pool initialization
//! - [`email`]: Email/SMTP configuration for sending notifications
//! - [`jwt`]: JWT authentication configuration
//! - [`rate_limit`]: API rate limiting configuration
//!
//! # Environment Variables
//!
//! Most configuration is loaded from environment variables. See each
//! submodule for specific variable names and their defaults.
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

pub mod cors;
pub mod database;
pub mod email;
pub mod jwt;
pub mod rate_limit;
