//! Authentication middleware and extractors.
//!
//! This module provides authentication and authorization extractors for Axum handlers.
//! It implements JWT-based authentication with permission-based access control.
//!
//! # Overview
//!
//! The authentication system uses JWT tokens that contain:
//!
//! - User ID and email
//! - School ID (for school-scoped users, None for system admins)
//! - Role IDs assigned to the user
//! - Permission names derived from the user's roles
//!
//! # Extractors
//!
//! ## `AuthUser`
//!
//! The base extractor that validates the JWT and provides access to the user's claims.
//!
//! ```ignore
//! async fn handler(auth_user: AuthUser) -> impl IntoResponse {
//!     let user_id = auth_user.user_id()?;
//!     let email = auth_user.email();
//!     // ...
//! }
//! ```
//!
//! ## Permission Extractors
//!
//! Pre-defined extractors that require specific permissions:
//!
//! ```ignore
//! // Requires "users:create" permission
//! async fn create_user(
//!     RequireUsersCreate(auth_user): RequireUsersCreate,
//! ) -> impl IntoResponse {
//!     // Handler only executes if user has permission
//! }
//! ```
//!
//! # Available Permission Extractors
//!
//! | Extractor | Required Permission |
//! |-----------|---------------------|
//! | `RequireUsersCreate` | `users:create` |
//! | `RequireUsersRead` | `users:read` |
//! | `RequireUsersUpdate` | `users:update` |
//! | `RequireUsersDelete` | `users:delete` |
//! | `RequireSchoolsCreate` | `schools:create` |
//! | `RequireSchoolsRead` | `schools:read` |
//! | `RequireSchoolsUpdate` | `schools:update` |
//! | `RequireSchoolsDelete` | `schools:delete` |
//! | ... and more |
//!
//! # Custom Permission Checks
//!
//! For more complex authorization logic, use the `AuthUser` methods:
//!
//! ```ignore
//! async fn handler(auth_user: AuthUser) -> Result<impl IntoResponse, AppError> {
//!     // Check single permission
//!     if !auth_user.has_permission("custom:permission") {
//!         return Err(AppError::forbidden("Access denied".to_string()));
//!     }
//!
//!     // Check multiple permissions (any)
//!     if auth_user.has_any_permission(&["admin:read", "super:read"]) {
//!         // User has at least one of these permissions
//!     }
//!
//!     // Check multiple permissions (all)
//!     if auth_user.has_all_permissions(&["users:read", "users:update"]) {
//!         // User has all of these permissions
//!     }
//!
//!     Ok(Json("Success"))
//! }
//! ```
//!
//! # School Scoping
//!
//! For school-scoped operations, check the user's school:
//!
//! ```ignore
//! async fn handler(auth_user: AuthUser) -> Result<impl IntoResponse, AppError> {
//!     if let Some(school_id) = auth_user.school_id() {
//!         // User is scoped to a specific school
//!         // Only return data for this school
//!     } else {
//!         // User is a system admin with access to all schools
//!     }
//!     Ok(Json("Success"))
//! }
//! ```

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use chalkbyte_auth::{Claims, verify_token};
use chalkbyte_core::AppError;
use chalkbyte_models::ids::{RoleId, SchoolId, UserId};
use uuid::Uuid;

use crate::state::AppState;

/// Extractor that validates JWT and provides the authenticated user's claims.
///
/// This is the primary authentication extractor. It validates the `Authorization`
/// header, verifies the JWT signature and expiration, and provides access to
/// the user's claims including roles, permissions, and school scope.
///
/// # Usage
///
/// ```ignore
/// use crate::middleware::auth::AuthUser;
///
/// async fn protected_handler(auth_user: AuthUser) -> impl IntoResponse {
///     // Access user information
///     let user_id = auth_user.user_id()?;
///     let school_id = auth_user.school_id();
///
///     // Check permissions
///     if auth_user.has_permission("admin:write") {
///         // Perform admin operation
///     }
///
///     Ok(Json("Success"))
/// }
/// ```
///
/// # Errors
///
/// Returns `401 Unauthorized` if:
/// - The `Authorization` header is missing
/// - The header format is invalid (not `Bearer <token>`)
/// - The JWT signature is invalid
/// - The JWT has expired
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl AuthUser {
    /// Checks if the user has a specific permission.
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission name to check (e.g., "users:create")
    ///
    /// # Returns
    ///
    /// `true` if the user has the permission, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if auth_user.has_permission("users:delete") {
    ///     // User can delete users
    /// }
    /// ```
    #[must_use]
    pub fn has_permission(&self, permission: &str) -> bool {
        self.0.permissions.contains(&permission.to_string())
    }

    /// Checks if the user has any of the specified permissions.
    ///
    /// # Arguments
    ///
    /// * `permissions` - A slice of permission names to check
    ///
    /// # Returns
    ///
    /// `true` if the user has at least one of the permissions, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if auth_user.has_any_permission(&["admin:read", "super:read"]) {
    ///     // User has elevated read access
    /// }
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Checks if the user has all of the specified permissions.
    ///
    /// # Arguments
    ///
    /// * `permissions` - A slice of permission names to check
    ///
    /// # Returns
    ///
    /// `true` if the user has all of the permissions, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if auth_user.has_all_permissions(&["users:read", "users:update", "users:delete"]) {
    ///     // User has full user management access
    /// }
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }

    /// Checks if the user has a specific role by ID.
    ///
    /// # Arguments
    ///
    /// * `role_id` - The RoleId to check
    ///
    /// # Returns
    ///
    /// `true` if the user has the role, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::modules::users::model::system_roles;
    ///
    /// if auth_user.has_role(&system_roles::SYSTEM_ADMIN) {
    ///     // User is a system admin
    /// }
    /// ```
    #[must_use]
    pub fn has_role(&self, role_id: &RoleId) -> bool {
        self.0.role_ids.contains(&role_id.into_inner())
    }

    /// Checks if the user has any of the specified roles.
    ///
    /// # Arguments
    ///
    /// * `role_ids` - A slice of RoleIds to check
    ///
    /// # Returns
    ///
    /// `true` if the user has at least one of the roles, `false` otherwise.
    #[allow(dead_code)]
    #[must_use]
    pub fn has_any_role(&self, role_ids: &[RoleId]) -> bool {
        role_ids
            .iter()
            .any(|r| self.0.role_ids.contains(&r.into_inner()))
    }

    /// Gets the user's school ID.
    ///
    /// # Returns
    ///
    /// - `Some(school_id)` for school-scoped users (admins, teachers, students)
    /// - `None` for system administrators who have access to all schools
    ///
    /// # Example
    ///
    /// ```ignore
    /// match auth_user.school_id() {
    ///     Some(school_id) => {
    ///         // Filter results to this school
    ///         query_users_by_school(school_id)
    ///     }
    ///     None => {
    ///         // System admin: return all users
    ///         query_all_users()
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn school_id(&self) -> Option<SchoolId> {
        self.0.school_id.map(SchoolId::from)
    }

    /// Gets the user ID as a UserId.
    ///
    /// # Returns
    ///
    /// The user's UserId on success.
    ///
    /// # Errors
    ///
    /// Returns an unauthorized error if the user ID in the token is not a valid UUID.
    /// This should not happen with properly issued tokens.
    pub fn user_id(&self) -> Result<UserId, AppError> {
        Uuid::parse_str(&self.0.sub)
            .map(UserId::from)
            .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))
    }

    /// Gets the user's email address.
    ///
    /// # Returns
    ///
    /// A reference to the user's email string.
    #[allow(dead_code)]
    #[must_use]
    pub fn email(&self) -> &str {
        &self.0.email
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing authorization header".to_string()))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::unauthorized("Invalid authorization header format".to_string())
        })?;

        let claims = verify_token(token, &state.jwt_config)?;

        Ok(AuthUser(claims))
    }
}

/// Helper macro to create permission check extractors for common permissions.
///
/// This macro generates a new extractor type that wraps `AuthUser` and
/// automatically checks for a specific permission before the handler executes.
///
/// # Usage
///
/// ```ignore
/// // Define a custom permission extractor
/// require_permission!(RequireCustomAccess, "custom:access");
///
/// // Use in a handler
/// async fn handler(RequireCustomAccess(auth_user): RequireCustomAccess) -> impl IntoResponse {
///     // Handler only executes if user has "custom:access" permission
/// }
/// ```
///
/// # Generated Type
///
/// The macro generates a tuple struct that wraps `AuthUser`:
///
/// ```ignore
/// pub struct RequireCustomAccess(pub AuthUser);
/// ```
///
/// The extractor automatically returns `403 Forbidden` if the user lacks
/// the required permission.
#[macro_export]
macro_rules! require_permission {
    ($name:ident, $permission:literal) => {
        #[allow(dead_code)]
        #[derive(Debug, Clone)]
        pub struct $name(pub $crate::middleware::auth::AuthUser);

        impl axum::extract::FromRequestParts<$crate::state::AppState> for $name {
            type Rejection = $crate::utils::errors::AppError;

            async fn from_request_parts(
                parts: &mut axum::http::request::Parts,
                state: &$crate::state::AppState,
            ) -> Result<Self, Self::Rejection> {
                let auth_user =
                    $crate::middleware::auth::AuthUser::from_request_parts(parts, state).await?;

                if !auth_user.has_permission($permission) {
                    return Err($crate::utils::errors::AppError::forbidden(format!(
                        "Access denied. Missing required permission: {}",
                        $permission
                    )));
                }

                Ok($name(auth_user))
            }
        }
    };
}

// =============================================================================
// Pre-defined permission extractors for common operations
// =============================================================================

// Users permissions
require_permission!(RequireUsersCreate, "users:create");
require_permission!(RequireUsersRead, "users:read");
require_permission!(RequireUsersUpdate, "users:update");
require_permission!(RequireUsersDelete, "users:delete");

// Schools permissions
require_permission!(RequireSchoolsCreate, "schools:create");
require_permission!(RequireSchoolsRead, "schools:read");
require_permission!(RequireSchoolsUpdate, "schools:update");
require_permission!(RequireSchoolsDelete, "schools:delete");

// Students permissions
require_permission!(RequireStudentsCreate, "students:create");
require_permission!(RequireStudentsRead, "students:read");
require_permission!(RequireStudentsUpdate, "students:update");
require_permission!(RequireStudentsDelete, "students:delete");

// Levels permissions
require_permission!(RequireLevelsCreate, "levels:create");
require_permission!(RequireLevelsRead, "levels:read");
require_permission!(RequireLevelsUpdate, "levels:update");
require_permission!(RequireLevelsDelete, "levels:delete");
require_permission!(RequireLevelsAssignStudents, "levels:assign_students");

// Branches permissions
require_permission!(RequireBranchesCreate, "branches:create");
require_permission!(RequireBranchesRead, "branches:read");
require_permission!(RequireBranchesUpdate, "branches:update");
require_permission!(RequireBranchesDelete, "branches:delete");
require_permission!(RequireBranchesAssignStudents, "branches:assign_students");

// Roles permissions
require_permission!(RequireRolesCreate, "roles:create");
require_permission!(RequireRolesRead, "roles:read");
require_permission!(RequireRolesUpdate, "roles:update");
require_permission!(RequireRolesDelete, "roles:delete");
require_permission!(RequireRolesAssign, "roles:assign");

// Reports permissions
require_permission!(RequireReportsView, "reports:view");
require_permission!(RequireReportsExport, "reports:export");

// Settings permissions
require_permission!(RequireSettingsRead, "settings:read");
require_permission!(RequireSettingsUpdate, "settings:update");

#[cfg(test)]
mod tests {
    use super::*;
    use chalkbyte_auth::Claims;

    fn create_test_claims(permissions: Vec<String>, role_ids: Vec<Uuid>) -> Claims {
        Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id: None,
            role_ids,
            permissions,
            exp: 9999999999,
            iat: 1234567890,
        }
    }

    #[test]
    fn test_has_permission() {
        let claims = create_test_claims(
            vec!["users:read".to_string(), "users:create".to_string()],
            vec![],
        );
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_permission("users:read"));
        assert!(auth_user.has_permission("users:create"));
        assert!(!auth_user.has_permission("users:delete"));
    }

    #[test]
    fn test_has_any_permission() {
        let claims = create_test_claims(vec!["users:read".to_string()], vec![]);
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_any_permission(&["users:read", "users:delete"]));
        assert!(!auth_user.has_any_permission(&["users:create", "users:delete"]));
    }

    #[test]
    fn test_has_all_permissions() {
        let claims = create_test_claims(
            vec![
                "users:read".to_string(),
                "users:create".to_string(),
                "users:update".to_string(),
            ],
            vec![],
        );
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_all_permissions(&["users:read", "users:create"]));
        assert!(!auth_user.has_all_permissions(&["users:read", "users:delete"]));
    }

    #[test]
    fn test_has_role() {
        let role_uuid = Uuid::new_v4();
        let role_id = RoleId::from(role_uuid);
        let claims = create_test_claims(vec![], vec![role_uuid]);
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_role(&role_id));
        assert!(!auth_user.has_role(&RoleId::from(Uuid::new_v4())));
    }

    #[test]
    fn test_has_any_role() {
        let role_uuid1 = Uuid::new_v4();
        let role_id1 = RoleId::from(role_uuid1);
        let role_id2 = RoleId::from(Uuid::new_v4());
        let claims = create_test_claims(vec![], vec![role_uuid1]);
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_any_role(&[role_id1, role_id2]));
        assert!(!auth_user.has_any_role(&[RoleId::from(Uuid::new_v4())]));
    }

    #[test]
    fn test_user_id() {
        let user_id = Uuid::new_v4();
        let claims = Claims {
            sub: user_id.to_string(),
            email: "test@example.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(auth_user.user_id().unwrap(), UserId::from(user_id));
    }

    #[test]
    fn test_school_id() {
        let school_uuid = Uuid::new_v4();
        let school_id = SchoolId::from(school_uuid);
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id: Some(school_uuid),
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(auth_user.school_id(), Some(school_id));
    }

    #[test]
    fn test_school_id_none_for_system_admin() {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "admin@example.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert!(auth_user.school_id().is_none());
    }

    #[test]
    fn test_email() {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "user@test.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(auth_user.email(), "user@test.com");
    }

    #[test]
    fn test_auth_user_clone() {
        let claims = create_test_claims(vec!["users:read".to_string()], vec![]);
        let auth_user = AuthUser(claims);
        let cloned = auth_user.clone();

        assert_eq!(auth_user.0.sub, cloned.0.sub);
        assert_eq!(auth_user.0.email, cloned.0.email);
    }

    #[test]
    fn test_auth_user_debug() {
        let claims = create_test_claims(vec![], vec![]);
        let auth_user = AuthUser(claims);
        let debug_str = format!("{:?}", auth_user);
        assert!(debug_str.contains("AuthUser"));
    }
}
