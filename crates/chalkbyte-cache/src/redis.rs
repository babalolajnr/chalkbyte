//! Redis cache client for distributed caching.
//!
//! Provides async Redis operations with JSON serialization for cached values.

use redis::{AsyncCommands, Client, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, instrument};

/// Redis cache client with connection pooling.
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
    default_ttl: Duration,
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("default_ttl", &self.default_ttl)
            .finish_non_exhaustive()
    }
}

/// Error type for cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Redis connection error: {0}")]
    Connection(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache miss")]
    Miss,
}

impl RedisCache {
    /// Creates a new Redis cache client.
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `default_ttl` - Default time-to-live for cached entries
    ///
    /// # Errors
    ///
    /// Returns `CacheError::Connection` if connection fails.
    pub async fn new(redis_url: &str, default_ttl: Duration) -> Result<Self, CacheError> {
        let client = Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;

        Ok(Self { conn, default_ttl })
    }

    /// Gets a cached value by key.
    ///
    /// Returns `None` if the key doesn't exist or deserialization fails.
    #[instrument(skip(self), fields(cache.operation = "GET"))]
    pub async fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.conn.clone();

        match conn.get::<_, Option<String>>(key).await {
            Ok(Some(value)) => {
                debug!(cache.key = %key, "Cache hit");
                match serde_json::from_str(&value) {
                    Ok(parsed) => Some(parsed),
                    Err(e) => {
                        error!(cache.key = %key, error = %e, "Failed to deserialize cached value");
                        None
                    }
                }
            }
            Ok(None) => {
                debug!(cache.key = %key, "Cache miss");
                None
            }
            Err(e) => {
                error!(cache.key = %key, error = %e, "Redis GET error");
                None
            }
        }
    }

    /// Sets a cached value with the default TTL.
    #[instrument(skip(self, value), fields(cache.operation = "SET"))]
    pub async fn set<T>(&self, key: &str, value: &T) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        self.set_with_ttl(key, value, self.default_ttl).await
    }

    /// Sets a cached value with a custom TTL.
    #[instrument(skip(self, value), fields(cache.operation = "SETEX"))]
    pub async fn set_with_ttl<T>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let mut conn = self.conn.clone();
        let json = serde_json::to_string(value)?;

        conn.set_ex::<_, _, ()>(key, json, ttl.as_secs()).await?;

        debug!(cache.key = %key, cache.ttl_secs = %ttl.as_secs(), "Cache set");

        Ok(())
    }

    /// Invalidates (deletes) a cached key.
    #[instrument(skip(self), fields(cache.operation = "DEL"))]
    pub async fn invalidate(&self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();

        conn.del::<_, ()>(key).await?;

        debug!(cache.key = %key, "Cache invalidated");

        Ok(())
    }

    /// Invalidates all keys matching a pattern.
    ///
    /// # Warning
    ///
    /// Uses SCAN which is safe for production, but may be slow with many keys.
    #[instrument(skip(self), fields(cache.operation = "SCAN_DEL"))]
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        let mut conn = self.conn.clone();
        let mut cursor: u64 = 0;
        let mut deleted: u64 = 0;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            if !keys.is_empty() {
                let count: u64 = conn.del(&keys).await?;
                deleted += count;
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        debug!(cache.pattern = %pattern, cache.deleted = %deleted, "Pattern invalidation complete");

        Ok(deleted)
    }

    /// Checks if a key exists in the cache.
    #[instrument(skip(self), fields(cache.operation = "EXISTS"))]
    pub async fn exists(&self, key: &str) -> bool {
        let mut conn = self.conn.clone();

        match conn.exists::<_, bool>(key).await {
            Ok(exists) => exists,
            Err(e) => {
                error!(cache.key = %key, error = %e, "Redis EXISTS error");
                false
            }
        }
    }

    /// Gets the remaining TTL for a key in seconds.
    ///
    /// Returns `None` if the key doesn't exist or has no TTL.
    #[instrument(skip(self), fields(cache.operation = "TTL"))]
    pub async fn ttl(&self, key: &str) -> Option<i64> {
        let mut conn = self.conn.clone();

        match conn.ttl::<_, i64>(key).await {
            Ok(ttl) if ttl > 0 => Some(ttl),
            Ok(_) => None, // -1 (no expiry) or -2 (doesn't exist)
            Err(e) => {
                error!(cache.key = %key, error = %e, "Redis TTL error");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i32,
        name: String,
    }

    // Integration tests require a running Redis instance
    // Run with: cargo test --features redis-tests

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn test_set_and_get() {
        let cache = RedisCache::new("redis://localhost:6379", Duration::from_secs(60))
            .await
            .unwrap();

        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        cache.set("test:key", &data).await.unwrap();

        let retrieved: Option<TestData> = cache.get("test:key").await;
        assert_eq!(retrieved, Some(data));

        cache.invalidate("test:key").await.unwrap();
    }
}
