//! # Chalkbyte Cache
//!
//! Redis-based caching utilities for the Chalkbyte API.
//!
//! This crate provides:
//! - Redis connection management
//! - Cache operations (get, set, delete, invalidate by prefix)
//! - Cache configuration from environment variables
//! - HTTP caching middleware (ETag, Cache-Control)
//! - Cache key generation utilities
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_cache::{CacheConfig, RedisCache};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = CacheConfig::from_env();
//!     let cache = RedisCache::new(&config.redis_url, Duration::from_secs(config.default_ttl_seconds))
//!         .await
//!         .unwrap();
//!
//!     // Set a value
//!     cache.set("key", &my_value).await.unwrap();
//!
//!     // Get a value
//!     let value: Option<MyType> = cache.get("key").await;
//! }
//! ```

pub mod config;
pub mod keys;
pub mod middleware;
pub mod redis;

pub use config::CacheConfig;
pub use keys::{hash_filters, invalidate};
pub use middleware::{
    CacheControlConfig, CacheableRoute, cache_control, cache_control_duration, etag_middleware,
};
pub use redis::{CacheError, RedisCache};
