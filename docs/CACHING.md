# Caching in Chalkbyte API

This document describes the caching implementation in the Chalkbyte API, which uses a two-layer approach:

1. **Redis Cache** - Server-side distributed caching for database query results
2. **HTTP Caching** - Client-side caching using `Cache-Control` and `ETag` headers

## Table of Contents

- [Environment Variables](#environment-variables)
- [Redis Caching](#redis-caching)
  - [Configuration](#configuration)
  - [Usage in Services](#usage-in-services)
  - [Cache Keys](#cache-keys)
  - [Cache Invalidation](#cache-invalidation)
- [HTTP Caching](#http-caching)
  - [Cache-Control Headers](#cache-control-headers)
  - [ETag Support](#etag-support)
  - [Route Configuration](#route-configuration)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Environment Variables

Add these to your `.env` file:

```bash
# Redis Configuration
REDIS_URL=redis://127.0.0.1:6379
CACHE_TTL_SECONDS=300        # Default TTL: 5 minutes
CACHE_PREFIX=chalkbyte       # Key prefix to avoid collisions
```

## Redis Caching

### Configuration

The Redis cache is initialized automatically in `AppState`. If Redis is unavailable, the application continues without caching (graceful degradation).

```rust
use chalkbyte_cache::{CacheConfig, RedisCache};

// Configuration is loaded from environment
let config = CacheConfig::from_env();

// Cache is optional in AppState
pub struct AppState {
    pub cache: Option<RedisCache>,
    // ...
}
```

### Usage in Services

```rust
use chalkbyte_cache::{RedisCache, keys};

impl SchoolService {
    pub async fn get_school_by_id(
        db: &PgPool,
        cache: Option<&RedisCache>,
        school_id: Uuid,
    ) -> Result<School, AppError> {
        let cache_key = keys::schools::by_id(school_id);

        // Try cache first
        if let Some(cache) = cache {
            if let Some(school) = cache.get::<School>(&cache_key).await {
                return Ok(school);
            }
        }

        // Cache miss - fetch from database
        let school = sqlx::query_as!(...)
            .fetch_one(db)
            .await?;

        // Store in cache
        if let Some(cache) = cache {
            let _ = cache.set(&cache_key, &school).await;
        }

        Ok(school)
    }
}
```

### Cache Keys

Use the `chalkbyte_cache::keys` module for consistent key generation:

```rust
use chalkbyte_cache::keys;

// School keys
keys::schools::by_id(school_id);           // chalkbyte:school:{id}
keys::schools::list(filters_hash);         // chalkbyte:schools:list:{hash}
keys::schools::full_info(school_id);       // chalkbyte:school:{id}:full

// User keys
keys::users::by_id(user_id);               // chalkbyte:user:{id}
keys::users::by_school(school_id, hash);   // chalkbyte:school:{id}:users:{hash}

// Level keys
keys::levels::by_id(level_id);             // chalkbyte:level:{id}
keys::levels::by_school(school_id);        // chalkbyte:school:{id}:levels

// Branch keys
keys::branches::by_id(branch_id);          // chalkbyte:branch:{id}
keys::branches::by_level(level_id);        // chalkbyte:level:{id}:branches

// Role keys
keys::roles::by_id(role_id);               // chalkbyte:role:{id}
keys::roles::user_roles(user_id);          // chalkbyte:user:{id}:roles
keys::roles::user_permissions(user_id);    // chalkbyte:user:{id}:permissions
```

### Cache Invalidation

Always invalidate cache on mutations (create, update, delete):

```rust
pub async fn delete_school(
    db: &PgPool,
    cache: Option<&RedisCache>,
    school_id: Uuid,
) -> Result<(), AppError> {
    // Delete from database
    sqlx::query!("DELETE FROM schools WHERE id = $1", school_id)
        .execute(db)
        .await?;

    // Invalidate cache
    if let Some(cache) = cache {
        // Invalidate specific key
        let _ = cache.invalidate(&keys::schools::by_id(school_id)).await;

        // Invalidate related list caches using pattern
        let _ = cache.invalidate_pattern(&keys::schools::invalidation_pattern()).await;
    }

    Ok(())
}
```

**Available invalidation patterns:**

```rust
keys::schools::invalidation_pattern()  // chalkbyte:school*
keys::users::invalidation_pattern()    // chalkbyte:user*
keys::levels::invalidation_pattern()   // chalkbyte:level*
keys::branches::invalidation_pattern() // chalkbyte:branch*
keys::roles::invalidation_pattern()    // chalkbyte:role*
```

### Invalidation Helpers

Use the `chalkbyte_cache::invalidate` module for simplified cache invalidation:

```rust
use chalkbyte_cache::invalidate;

// After creating/updating/deleting a school
invalidate::school(cache, Some(school_id)).await;

// After creating/updating/deleting a user
invalidate::user(cache, Some(user_id), Some(school_id)).await;

// After creating/updating/deleting a level
invalidate::level(cache, Some(level_id), Some(school_id)).await;

// After creating/updating/deleting a branch
invalidate::branch(cache, Some(branch_id), Some(level_id)).await;

// After creating/updating/deleting a role
invalidate::role(cache, Some(role_id)).await;

// After assigning/removing roles from a user
invalidate::user_roles(cache, user_id).await;
```

These helpers automatically invalidate:
- The specific entity cache (by ID)
- Related list caches (using pattern matching)
- Parent entity caches (e.g., school's user list when a user changes)

## Cache Freshness Strategy

### When to Invalidate

| Operation | What to Invalidate |
|-----------|-------------------|
| **Create** | List caches only (new entity should appear) |
| **Update** | Specific entity + list caches |
| **Delete** | Specific entity + list caches + related entities |

### Invalidation Matrix

| Entity | On Create | On Update | On Delete |
|--------|-----------|-----------|-----------|
| School | `invalidate::school(cache, Some(id))` | `invalidate::school(cache, Some(id))` | `invalidate::school(cache, Some(id))` |
| User | `invalidate::user(cache, Some(id), Some(school_id))` | `invalidate::user(cache, Some(id), Some(school_id))` | `invalidate::user(cache, Some(id), Some(school_id))` |
| Level | `invalidate::level(cache, Some(id), Some(school_id))` | `invalidate::level(cache, Some(id), Some(school_id))` | `invalidate::level(cache, Some(id), Some(school_id))` + branches |
| Branch | `invalidate::branch(cache, Some(id), Some(level_id))` | `invalidate::branch(cache, Some(id), Some(level_id))` | `invalidate::branch(cache, Some(id), Some(level_id))` |
| Role | `invalidate::role(cache, Some(id))` | `invalidate::role(cache, Some(id))` | `invalidate::role(cache, Some(id))` + user_roles |

### Example: Complete Service Pattern

```rust
use chalkbyte_cache::{RedisCache, invalidate, keys};

impl MyEntityService {
    // CREATE - invalidate lists
    pub async fn create(
        db: &PgPool,
        cache: Option<&RedisCache>,
        dto: CreateDto,
    ) -> Result<Entity, AppError> {
        let entity = sqlx::query_as!(...)
            .fetch_one(db)
            .await?;

        // Invalidate so new entity appears in lists
        invalidate::entity(cache, Some(entity.id), parent_id).await;

        Ok(entity)
    }

    // READ - cache on miss
    pub async fn get_by_id(
        db: &PgPool,
        cache: Option<&RedisCache>,
        id: Uuid,
    ) -> Result<Entity, AppError> {
        let cache_key = keys::entities::by_id(id);

        // Try cache first
        if let Some(cache) = cache {
            if let Some(entity) = cache.get::<Entity>(&cache_key).await {
                return Ok(entity);
            }
        }

        // Cache miss - fetch from DB
        let entity = sqlx::query_as!(...)
            .fetch_one(db)
            .await?;

        // Store in cache
        if let Some(cache) = cache {
            let _ = cache.set(&cache_key, &entity).await;
        }

        Ok(entity)
    }

    // UPDATE - invalidate specific + lists
    pub async fn update(
        db: &PgPool,
        cache: Option<&RedisCache>,
        id: Uuid,
        dto: UpdateDto,
    ) -> Result<Entity, AppError> {
        let entity = sqlx::query_as!(...)
            .fetch_one(db)
            .await?;

        // Invalidate old cached value and lists
        invalidate::entity(cache, Some(id), parent_id).await;

        Ok(entity)
    }

    // DELETE - invalidate specific + lists + related
    pub async fn delete(
        db: &PgPool,
        cache: Option<&RedisCache>,
        id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM entities WHERE id = $1", id)
            .execute(db)
            .await?;

        // Invalidate everything related
        invalidate::entity(cache, Some(id), parent_id).await;

        Ok(())
    }
}
```

### Avoiding Stale Data

1. **Always invalidate on mutations** - Never skip cache invalidation on create/update/delete
2. **Use short TTLs for volatile data** - 60 seconds for frequently changing data
3. **Invalidate parent caches** - When a child changes, parent lists may be stale
4. **Use pattern invalidation** - `invalidate_pattern("prefix:*")` for related caches
5. **Cache is optional** - Design services to work without cache (`Option<&RedisCache>`)

## HTTP Caching

### Cache-Control Headers

HTTP caching is configured per-route using middleware:

```rust
use chalkbyte_cache::{CacheControlConfig, cache_control};

// No caching (for sensitive endpoints like auth)
let no_cache = cache_control(CacheControlConfig::no_store());

// Private caching (browser only, short TTL)
let private_short = cache_control(CacheControlConfig::private(60).with_must_revalidate());

// Private caching (browser only, medium TTL)
let private_medium = cache_control(CacheControlConfig::private(300).with_must_revalidate());

// Public caching (CDN/proxy cacheable)
let public_cache = cache_control(CacheControlConfig::public(600));
```

**Configuration options:**

```rust
CacheControlConfig::public(max_age)     // public, max-age=X
CacheControlConfig::private(max_age)    // private, max-age=X
CacheControlConfig::no_cache()          // no-cache, must-revalidate
CacheControlConfig::no_store()          // no-store, no-cache

// Modifiers
.with_must_revalidate()                 // Adds must-revalidate
.with_s_maxage(seconds)                 // Adds s-maxage for CDNs
.with_stale_while_revalidate(seconds)   // Adds stale-while-revalidate
```

### ETag Support

The `etag_middleware` generates ETags from response bodies and handles `If-None-Match` headers:

```rust
use chalkbyte_cache::etag_middleware;

Router::new()
    .route("/api/schools", get(list_schools))
    .layer(middleware::from_fn(etag_middleware))
```

**How it works:**

1. Client makes request
2. Server generates response with `ETag` header (SHA-256 hash of body)
3. Client caches response with ETag
4. On subsequent requests, client sends `If-None-Match: "etag-value"`
5. If ETag matches, server returns `304 Not Modified` (no body)
6. Client uses cached response

### Route Configuration

Current route cache configuration in `router.rs`:

| Route | Cache-Control | ETag | Rationale |
|-------|--------------|------|-----------|
| `/api/auth/*` | `no-store` | No | Sensitive authentication data |
| `/api/mfa/*` | `no-store` | No | Sensitive MFA data |
| `/api/users/*` | `private, max-age=60, must-revalidate` | Yes | User data changes frequently |
| `/api/schools/*` | `private, max-age=300, must-revalidate` | Yes | School data is more stable |
| `/api/levels/*` | `private, max-age=300, must-revalidate` | Yes | Level data is stable |
| `/api/branches/*` | `private, max-age=300, must-revalidate` | Yes | Branch data is stable |
| `/api/roles/*` | `private, max-age=300, must-revalidate` | Yes | Roles rarely change |
| `/api/students/*` | `private, max-age=60, must-revalidate` | Yes | Student data changes frequently |

## Best Practices

### What to Cache

✅ **Cache:**
- School data (by ID, lists)
- Level and branch lookups
- Role and permission data
- User profiles (not credentials)
- Paginated list results (with filter hash in key)

❌ **Don't Cache:**
- Authentication tokens/sessions
- Password hashes
- MFA secrets
- Rapidly changing counters
- Data that changes on every request

### TTL Guidelines

| Data Type | Recommended TTL |
|-----------|----------------|
| Static config (roles) | 5-10 minutes |
| Entity lookups (school by ID) | 5 minutes |
| List queries | 1-2 minutes |
| User profiles | 1 minute |
| Counts/stats | 30 seconds |

### Cache Key Best Practices

1. **Include scope in keys** for multi-tenant data:
   ```rust
   format!("school:{}:users:{}", school_id, filters_hash)
   ```

2. **Hash filter parameters** for list queries:
   ```rust
   use chalkbyte_cache::keys::hash_filters;
   let key = keys::schools::list(&hash_filters(&filters));
   ```

3. **Use consistent prefixes** via the `keys` module

### Handling Cache Failures

The cache is designed to fail gracefully:

```rust
// Cache operations return Result, but failures are logged and ignored
if let Some(cache) = cache {
    if let Err(e) = cache.set(&key, &value).await {
        warn!(error = %e, "Failed to cache value");
        // Continue without caching - application still works
    }
}
```

## Troubleshooting

### Redis Connection Issues

**Symptom:** "Failed to connect to Redis" warning at startup

**Solutions:**
1. Verify Redis is running: `redis-cli ping`
2. Check `REDIS_URL` environment variable
3. Verify network connectivity to Redis host
4. Application continues without caching if Redis is unavailable

### Cache Not Working

**Checklist:**
1. Verify Redis is connected (check startup logs)
2. Confirm `state.cache.is_some()` in your handler
3. Check cache key is correct using `redis-cli KEYS "chalkbyte:*"`
4. Verify TTL hasn't expired: `redis-cli TTL "your-key"`

### Stale Data

**Symptom:** Old data returned after updates

**Solutions:**
1. Ensure cache invalidation is called on mutations
2. Use pattern invalidation for related data
3. Check if multiple services need coordinated invalidation
4. Consider shorter TTLs for frequently updated data

### Monitoring Cache

```bash
# Check all Chalkbyte keys
redis-cli KEYS "chalkbyte:*"

# Check specific key TTL
redis-cli TTL "chalkbyte:school:uuid-here"

# Get cached value
redis-cli GET "chalkbyte:school:uuid-here"

# Clear all Chalkbyte cache
redis-cli KEYS "chalkbyte:*" | xargs redis-cli DEL

# Monitor cache operations in real-time
redis-cli MONITOR | grep chalkbyte
```

### HTTP Cache Headers

To verify HTTP cache headers:

```bash
# Check response headers
curl -I -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/schools

# Test ETag with If-None-Match
curl -H "Authorization: Bearer $TOKEN" \
     -H "If-None-Match: \"your-etag\"" \
     http://localhost:3000/api/schools
# Should return 304 Not Modified if data unchanged
```
