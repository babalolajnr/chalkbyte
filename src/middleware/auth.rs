use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::modules::auth::model::Claims;
use crate::state::AppState;
use crate::utils::errors::AppError;
use crate::utils::jwt::verify_token;

/// Extractor that validates JWT and provides the authenticated user's claims.
/// Claims now include role_ids, permissions, and school_id for permission-based access control.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl AuthUser {
    /// Check if the user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.0.permissions.contains(&permission.to_string())
    }

    /// Check if the user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Check if the user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }

    /// Check if the user has a specific role by ID
    pub fn has_role(&self, role_id: &uuid::Uuid) -> bool {
        self.0.role_ids.contains(role_id)
    }

    /// Check if the user has any of the specified roles
    pub fn has_any_role(&self, role_ids: &[uuid::Uuid]) -> bool {
        role_ids.iter().any(|r| self.has_role(r))
    }

    /// Get the user's school_id (None for system admins)
    pub fn school_id(&self) -> Option<uuid::Uuid> {
        self.0.school_id
    }

    /// Get the user ID as UUID
    pub fn user_id(&self) -> Result<uuid::Uuid, AppError> {
        uuid::Uuid::parse_str(&self.0.sub)
            .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))
    }

    /// Get the user's email
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
/// This provides type-safe permission checking at compile time.
#[macro_export]
macro_rules! require_permission {
    ($name:ident, $permission:literal) => {
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

// Pre-defined permission extractors for common operations

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
    use crate::modules::auth::model::Claims;
    use uuid::Uuid;

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
        let role_id = Uuid::new_v4();
        let claims = create_test_claims(vec![], vec![role_id]);
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_role(&role_id));
        assert!(!auth_user.has_role(&Uuid::new_v4()));
    }

    #[test]
    fn test_has_any_role() {
        let role_id1 = Uuid::new_v4();
        let role_id2 = Uuid::new_v4();
        let claims = create_test_claims(vec![], vec![role_id1]);
        let auth_user = AuthUser(claims);

        assert!(auth_user.has_any_role(&[role_id1, role_id2]));
        assert!(!auth_user.has_any_role(&[Uuid::new_v4()]));
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

        assert_eq!(auth_user.user_id().unwrap(), user_id);
    }

    #[test]
    fn test_school_id() {
        let school_id = Uuid::new_v4();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id: Some(school_id),
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(auth_user.school_id(), Some(school_id));
    }
}
