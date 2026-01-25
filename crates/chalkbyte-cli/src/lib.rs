//! # Chalkbyte CLI
//!
//! Database seeding utilities for Chalkbyte testing and development.
//!
//! This library crate provides the seeding functionality used by the CLI binary.
//!
//! ## Usage
//!
//! ```ignore
//! use chalkbyte_cli::seeder::{seed_all, SeedConfig};
//!
//! let config = SeedConfig::new(10); // 10 schools with defaults
//! seed_all(&pool, config).await?;
//! ```

pub mod seeder;
