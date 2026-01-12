//! Cache key generation and invalidation utilities.
//!
//! Provides consistent cache key generation and invalidation helpers across the application.

use crate::RedisCache;
use tracing::warn;
use uuid::Uuid;

/// Prefix for all cache keys to avoid collisions with other Redis users.
const CACHE_PREFIX: &str = "chalkbyte";

/// Builds a cache key with the standard prefix.
fn build_key(parts: &[&str]) -> String {
    format!("{}:{}", CACHE_PREFIX, parts.join(":"))
}

/// Cache keys for school-related data.
pub mod schools {
    use super::*;

    /// Key for a single school by ID.
    pub fn by_id(school_id: Uuid) -> String {
        build_key(&["school", &school_id.to_string()])
    }

    /// Key for school list with filters hash.
    pub fn list(filters_hash: &str) -> String {
        build_key(&["schools", "list", filters_hash])
    }

    /// Key for school full info.
    pub fn full_info(school_id: Uuid) -> String {
        build_key(&["school", &school_id.to_string(), "full"])
    }

    /// Pattern to invalidate all school-related keys.
    pub fn invalidation_pattern() -> String {
        format!("{}:school*", CACHE_PREFIX)
    }
}

/// Cache keys for user-related data.
pub mod users {
    use super::*;

    /// Key for a single user by ID.
    pub fn by_id(user_id: Uuid) -> String {
        build_key(&["user", &user_id.to_string()])
    }

    /// Key for user list with filters hash.
    pub fn list(filters_hash: &str) -> String {
        build_key(&["users", "list", filters_hash])
    }

    /// Key for users by school.
    pub fn by_school(school_id: Uuid, filters_hash: &str) -> String {
        build_key(&["school", &school_id.to_string(), "users", filters_hash])
    }

    /// Pattern to invalidate all user-related keys.
    pub fn invalidation_pattern() -> String {
        format!("{}:user*", CACHE_PREFIX)
    }

    /// Pattern to invalidate users for a specific school.
    pub fn school_invalidation_pattern(school_id: Uuid) -> String {
        format!("{}:school:{}:users*", CACHE_PREFIX, school_id)
    }
}

/// Cache keys for level-related data.
pub mod levels {
    use super::*;

    /// Key for a single level by ID.
    pub fn by_id(level_id: Uuid) -> String {
        build_key(&["level", &level_id.to_string()])
    }

    /// Key for level list with filters hash.
    pub fn list(filters_hash: &str) -> String {
        build_key(&["levels", "list", filters_hash])
    }

    /// Key for levels by school.
    pub fn by_school(school_id: Uuid) -> String {
        build_key(&["school", &school_id.to_string(), "levels"])
    }

    /// Pattern to invalidate all level-related keys.
    pub fn invalidation_pattern() -> String {
        format!("{}:level*", CACHE_PREFIX)
    }
}

/// Cache keys for branch-related data.
pub mod branches {
    use super::*;

    /// Key for a single branch by ID.
    pub fn by_id(branch_id: Uuid) -> String {
        build_key(&["branch", &branch_id.to_string()])
    }

    /// Key for branch list with filters hash.
    pub fn list(filters_hash: &str) -> String {
        build_key(&["branches", "list", filters_hash])
    }

    /// Key for branches by level.
    pub fn by_level(level_id: Uuid) -> String {
        build_key(&["level", &level_id.to_string(), "branches"])
    }

    /// Pattern to invalidate all branch-related keys.
    pub fn invalidation_pattern() -> String {
        format!("{}:branch*", CACHE_PREFIX)
    }
}

/// Cache keys for role-related data.
pub mod roles {
    use super::*;

    /// Key for a single role by ID.
    pub fn by_id(role_id: Uuid) -> String {
        build_key(&["role", &role_id.to_string()])
    }

    /// Key for all roles list.
    pub fn list() -> String {
        build_key(&["roles", "list"])
    }

    /// Key for user's roles.
    pub fn user_roles(user_id: Uuid) -> String {
        build_key(&["user", &user_id.to_string(), "roles"])
    }

    /// Key for user's permissions.
    pub fn user_permissions(user_id: Uuid) -> String {
        build_key(&["user", &user_id.to_string(), "permissions"])
    }

    /// Pattern to invalidate all role-related keys.
    pub fn invalidation_pattern() -> String {
        format!("{}:role*", CACHE_PREFIX)
    }
}

/// Generates a hash from filter parameters for cache key uniqueness.
///
/// Uses a simple hash to create a short, consistent key component from
/// arbitrary filter parameters.
pub fn hash_filters<T: std::hash::Hash>(filters: &T) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    filters.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Cache invalidation helper for common operations.
///
/// This module provides high-level invalidation functions that handle
/// all related cache keys for a given entity type.
pub mod invalidate {
    use super::*;

    /// Invalidate all school-related caches.
    ///
    /// Call this after creating, updating, or deleting a school.
    pub async fn school(cache: Option<&RedisCache>, school_id: Option<Uuid>) {
        let Some(cache) = cache else { return };

        // Invalidate specific school if ID provided
        if let Some(id) = school_id {
            if let Err(e) = cache.invalidate(&schools::by_id(id)).await {
                warn!(error = %e, school_id = %id, "Failed to invalidate school cache");
            }
            if let Err(e) = cache.invalidate(&schools::full_info(id)).await {
                warn!(error = %e, school_id = %id, "Failed to invalidate school full_info cache");
            }
        }

        // Always invalidate list caches
        if let Err(e) = cache
            .invalidate_pattern(&schools::invalidation_pattern())
            .await
        {
            warn!(error = %e, "Failed to invalidate school list caches");
        }
    }

    /// Invalidate all user-related caches.
    ///
    /// Call this after creating, updating, or deleting a user.
    pub async fn user(cache: Option<&RedisCache>, user_id: Option<Uuid>, school_id: Option<Uuid>) {
        let Some(cache) = cache else { return };

        // Invalidate specific user if ID provided
        if let Some(id) = user_id {
            if let Err(e) = cache.invalidate(&users::by_id(id)).await {
                warn!(error = %e, user_id = %id, "Failed to invalidate user cache");
            }
        }

        // Invalidate school's user list if school_id provided
        if let Some(sid) = school_id {
            if let Err(e) = cache
                .invalidate_pattern(&users::school_invalidation_pattern(sid))
                .await
            {
                warn!(error = %e, school_id = %sid, "Failed to invalidate school users cache");
            }
        }

        // Invalidate general user list caches
        if let Err(e) = cache
            .invalidate_pattern(&users::invalidation_pattern())
            .await
        {
            warn!(error = %e, "Failed to invalidate user list caches");
        }
    }

    /// Invalidate all level-related caches.
    ///
    /// Call this after creating, updating, or deleting a level.
    pub async fn level(
        cache: Option<&RedisCache>,
        level_id: Option<Uuid>,
        school_id: Option<Uuid>,
    ) {
        let Some(cache) = cache else { return };

        // Invalidate specific level if ID provided
        if let Some(id) = level_id {
            if let Err(e) = cache.invalidate(&levels::by_id(id)).await {
                warn!(error = %e, level_id = %id, "Failed to invalidate level cache");
            }
        }

        // Invalidate school's level list if school_id provided
        if let Some(sid) = school_id {
            if let Err(e) = cache.invalidate(&levels::by_school(sid)).await {
                warn!(error = %e, school_id = %sid, "Failed to invalidate school levels cache");
            }
        }

        // Invalidate general level list caches
        if let Err(e) = cache
            .invalidate_pattern(&levels::invalidation_pattern())
            .await
        {
            warn!(error = %e, "Failed to invalidate level list caches");
        }
    }

    /// Invalidate all branch-related caches.
    ///
    /// Call this after creating, updating, or deleting a branch.
    pub async fn branch(
        cache: Option<&RedisCache>,
        branch_id: Option<Uuid>,
        level_id: Option<Uuid>,
    ) {
        let Some(cache) = cache else { return };

        // Invalidate specific branch if ID provided
        if let Some(id) = branch_id {
            if let Err(e) = cache.invalidate(&branches::by_id(id)).await {
                warn!(error = %e, branch_id = %id, "Failed to invalidate branch cache");
            }
        }

        // Invalidate level's branch list if level_id provided
        if let Some(lid) = level_id {
            if let Err(e) = cache.invalidate(&branches::by_level(lid)).await {
                warn!(error = %e, level_id = %lid, "Failed to invalidate level branches cache");
            }
        }

        // Invalidate general branch list caches
        if let Err(e) = cache
            .invalidate_pattern(&branches::invalidation_pattern())
            .await
        {
            warn!(error = %e, "Failed to invalidate branch list caches");
        }
    }

    /// Invalidate all role-related caches.
    ///
    /// Call this after creating, updating, or deleting a role.
    pub async fn role(cache: Option<&RedisCache>, role_id: Option<Uuid>) {
        let Some(cache) = cache else { return };

        // Invalidate specific role if ID provided
        if let Some(id) = role_id {
            if let Err(e) = cache.invalidate(&roles::by_id(id)).await {
                warn!(error = %e, role_id = %id, "Failed to invalidate role cache");
            }
        }

        // Invalidate role list cache
        if let Err(e) = cache.invalidate(&roles::list()).await {
            warn!(error = %e, "Failed to invalidate roles list cache");
        }

        // Invalidate all role-related caches
        if let Err(e) = cache
            .invalidate_pattern(&roles::invalidation_pattern())
            .await
        {
            warn!(error = %e, "Failed to invalidate role caches");
        }
    }

    /// Invalidate user's role and permission caches.
    ///
    /// Call this after assigning or removing roles from a user.
    pub async fn user_roles(cache: Option<&RedisCache>, user_id: Uuid) {
        let Some(cache) = cache else { return };

        if let Err(e) = cache.invalidate(&roles::user_roles(user_id)).await {
            warn!(error = %e, user_id = %user_id, "Failed to invalidate user roles cache");
        }
        if let Err(e) = cache.invalidate(&roles::user_permissions(user_id)).await {
            warn!(error = %e, user_id = %user_id, "Failed to invalidate user permissions cache");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_school_key_generation() {
        let id = Uuid::nil();
        let key = schools::by_id(id);
        assert!(key.starts_with("chalkbyte:school:"));
        assert!(key.contains(&id.to_string()));
    }

    #[test]
    fn test_user_key_generation() {
        let id = Uuid::nil();
        let key = users::by_id(id);
        assert!(key.starts_with("chalkbyte:user:"));
    }

    #[test]
    fn test_hash_filters_consistency() {
        let filters = ("test", 123, true);
        let hash1 = hash_filters(&filters);
        let hash2 = hash_filters(&filters);
        assert_eq!(hash1, hash2);
    }
}
