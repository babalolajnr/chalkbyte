//! Role-based authorization middleware for Axum
//!
//! This module provides permission-based access control using JWT-embedded
//! permissions. Since permissions are now embedded in the JWT token during login,
//! most authorization checks can be done without database queries.
//!
//! For operations that require fresh permission data (e.g., after role changes),
//! use the database-backed functions explicitly.

use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::modules::roles::service as roles_service;
use crate::modules::users::model::system_roles;
use chalkbyte_core::AppError;

use crate::modules::users::service::UserService;
use crate::state::AppState;

// Re-export auth helpers for convenience
pub use crate::utils::auth_helpers::get_admin_school_id;

// ============================================================================
// JWT-Based Permission Checking (No DB queries - uses embedded permissions)
// ============================================================================

/// Middleware function that checks if the user has a specific permission from JWT claims.
/// This is the preferred method as it doesn't require database queries.
#[allow(dead_code)]
pub async fn require_permission_from_jwt(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    permission_name: &str,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;

    if !auth_user.has_permission(permission_name) {
        return Err(AppError::forbidden(format!(
            "Access denied. Missing required permission: {}",
            permission_name
        )));
    }

    req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Middleware function that checks if the user has any of the specified permissions from JWT.
#[allow(dead_code)]
pub async fn require_any_permission_from_jwt(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    permissions: &[&str],
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;

    if !auth_user.has_any_permission(permissions) {
        return Err(AppError::forbidden(format!(
            "Access denied. Missing required permissions: {:?}",
            permissions
        )));
    }

    req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

// ============================================================================
// Database-Backed Permission Checking (Fresh data, use when needed)
// ============================================================================

/// Middleware function that checks if the authenticated user has one of the required roles.
/// Uses database to get fresh role data.
pub async fn require_roles(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    allowed_role_ids: Vec<Uuid>,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;

    let user_id = auth_user.user_id()?;

    // Check if user has any of the allowed roles (from database for fresh data)
    let has_role = UserService::user_has_any_role(&state.db, user_id, &allowed_role_ids).await?;

    if !has_role {
        return Err(AppError::forbidden(
            "Access denied. You do not have the required role.".to_string(),
        ));
    }

    req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Middleware function that checks if the user has a specific permission.
/// Uses database for fresh permission data - use when permissions may have changed.
#[allow(dead_code)]
pub async fn require_permission(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    permission_name: &str,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;

    let user_id = auth_user.user_id()?;

    // Check permission from database for fresh data
    let has_permission =
        roles_service::user_has_permission(&state.db, user_id, permission_name).await?;

    if !has_permission {
        return Err(AppError::forbidden(format!(
            "Access denied. Missing required permission: {}",
            permission_name
        )));
    }

    req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

// ============================================================================
// Role-Based Middleware Helpers (Database-backed for backward compatibility)
// ============================================================================

/// Helper function for system admin only routes
pub async fn require_system_admin(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    match require_roles(State(state), req, next, vec![system_roles::SYSTEM_ADMIN]).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Helper function for school admin routes (both SystemAdmin and Admin allowed)
pub async fn require_admin(State(state): State<AppState>, req: Request, next: Next) -> Response {
    match require_roles(
        State(state),
        req,
        next,
        vec![system_roles::SYSTEM_ADMIN, system_roles::ADMIN],
    )
    .await
    {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Helper function for teacher routes (SystemAdmin, Admin, and Teacher allowed)
#[allow(dead_code)]
pub async fn require_teacher(State(state): State<AppState>, req: Request, next: Next) -> Response {
    match require_roles(
        State(state),
        req,
        next,
        vec![
            system_roles::SYSTEM_ADMIN,
            system_roles::ADMIN,
            system_roles::TEACHER,
        ],
    )
    .await
    {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

// ============================================================================
// Extractors (Use JWT-embedded data by default)
// ============================================================================

/// Simple role checker extractor for system admin requirement.
/// Uses JWT-embedded role_ids for fast checking.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RequireSystemAdmin(pub Uuid);

impl FromRequestParts<AppState> for RequireSystemAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = auth_user.user_id()?;

        // Check from JWT claims first (fast path)
        if auth_user.has_role(&system_roles::SYSTEM_ADMIN) {
            return Ok(RequireSystemAdmin(user_id));
        }

        // Fallback to database check for fresh data
        let is_sys_admin = UserService::is_system_admin(&state.db, user_id).await?;

        if !is_sys_admin {
            return Err(AppError::forbidden(
                "Access denied. Only system administrators can access this resource.".to_string(),
            ));
        }

        Ok(RequireSystemAdmin(user_id))
    }
}

/// Extractor for admin-level access (SystemAdmin or Admin).
/// Uses JWT-embedded role_ids for fast checking.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub Uuid);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = auth_user.user_id()?;

        // Check from JWT claims first (fast path)
        if auth_user.has_any_role(&[system_roles::SYSTEM_ADMIN, system_roles::ADMIN]) {
            return Ok(RequireAdmin(user_id));
        }

        // Fallback to database check for fresh data
        let has_role = UserService::user_has_any_role(
            &state.db,
            user_id,
            &[system_roles::SYSTEM_ADMIN, system_roles::ADMIN],
        )
        .await?;

        if !has_role {
            return Err(AppError::forbidden(
                "Access denied. Administrator privileges required.".to_string(),
            ));
        }

        Ok(RequireAdmin(user_id))
    }
}

/// Extractor for teacher-level access (SystemAdmin, Admin, or Teacher).
/// Uses JWT-embedded role_ids for fast checking.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RequireTeacher(pub Uuid);

impl FromRequestParts<AppState> for RequireTeacher {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = auth_user.user_id()?;

        // Check from JWT claims first (fast path)
        if auth_user.has_any_role(&[
            system_roles::SYSTEM_ADMIN,
            system_roles::ADMIN,
            system_roles::TEACHER,
        ]) {
            return Ok(RequireTeacher(user_id));
        }

        // Fallback to database check for fresh data
        let has_role = UserService::user_has_any_role(
            &state.db,
            user_id,
            &[
                system_roles::SYSTEM_ADMIN,
                system_roles::ADMIN,
                system_roles::TEACHER,
            ],
        )
        .await?;

        if !has_role {
            return Err(AppError::forbidden(
                "Access denied. Teacher privileges required.".to_string(),
            ));
        }

        Ok(RequireTeacher(user_id))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the user ID from auth claims
#[allow(dead_code)]
pub fn get_user_id_from_auth(auth_user: &AuthUser) -> Result<Uuid, AppError> {
    auth_user.user_id()
}

/// Check if user has any of the specified roles using JWT claims (fast, no DB)
#[allow(dead_code)]
pub fn check_user_has_any_role_jwt(auth_user: &AuthUser, role_ids: &[Uuid]) -> bool {
    auth_user.has_any_role(role_ids)
}

/// Check if user has a specific permission using JWT claims (fast, no DB)
#[allow(dead_code)]
pub fn check_user_has_permission_jwt(auth_user: &AuthUser, permission_name: &str) -> bool {
    auth_user.has_permission(permission_name)
}

/// Helper function to check if a user has any of the specified roles (database query)
#[allow(dead_code)]
pub async fn check_user_has_any_role(
    db: &sqlx::PgPool,
    user_id: Uuid,
    role_ids: &[Uuid],
) -> Result<bool, AppError> {
    UserService::user_has_any_role(db, user_id, role_ids).await
}

/// Helper function to check if a user has a specific permission (database query)
#[allow(dead_code)]
pub async fn check_user_has_permission(
    db: &sqlx::PgPool,
    user_id: Uuid,
    permission_name: &str,
) -> Result<bool, AppError> {
    roles_service::user_has_permission(db, user_id, permission_name).await
}

/// Check if user is a system admin (database query)
#[allow(dead_code)]
pub async fn is_system_admin(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    UserService::is_system_admin(db, user_id).await
}

/// Check if user is a system admin using JWT claims (fast, no DB)
pub fn is_system_admin_jwt(auth_user: &AuthUser) -> bool {
    auth_user.has_role(&system_roles::SYSTEM_ADMIN)
}

/// Check if user is an admin (school admin or system admin) using database
#[allow(dead_code)]
pub async fn is_admin(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    UserService::user_has_any_role(
        db,
        user_id,
        &[system_roles::SYSTEM_ADMIN, system_roles::ADMIN],
    )
    .await
}

/// Check if user is an admin using JWT claims (fast, no DB)
#[allow(dead_code)]
pub fn is_admin_jwt(auth_user: &AuthUser) -> bool {
    auth_user.has_any_role(&[system_roles::SYSTEM_ADMIN, system_roles::ADMIN])
}

/// Check if user is at least a teacher (teacher, admin, or system admin) using database
#[allow(dead_code)]
pub async fn is_teacher_or_above(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    UserService::user_has_any_role(
        db,
        user_id,
        &[
            system_roles::SYSTEM_ADMIN,
            system_roles::ADMIN,
            system_roles::TEACHER,
        ],
    )
    .await
}

/// Check if user is at least a teacher using JWT claims (fast, no DB)
#[allow(dead_code)]
pub fn is_teacher_or_above_jwt(auth_user: &AuthUser) -> bool {
    auth_user.has_any_role(&[
        system_roles::SYSTEM_ADMIN,
        system_roles::ADMIN,
        system_roles::TEACHER,
    ])
}

/// Get the school_id from auth user (from JWT claims)
#[allow(dead_code)]
pub fn get_school_id_from_auth(auth_user: &AuthUser) -> Option<Uuid> {
    auth_user.school_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::model::Claims;

    fn create_test_auth_user(role_ids: Vec<Uuid>, permissions: Vec<String>) -> AuthUser {
        AuthUser(Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id: None,
            role_ids,
            permissions,
            exp: 9999999999,
            iat: 1234567890,
        })
    }

    #[test]
    fn test_system_role_ids() {
        assert_eq!(
            system_roles::SYSTEM_ADMIN.to_string(),
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(
            system_roles::ADMIN.to_string(),
            "00000000-0000-0000-0000-000000000002"
        );
        assert_eq!(
            system_roles::TEACHER.to_string(),
            "00000000-0000-0000-0000-000000000003"
        );
        assert_eq!(
            system_roles::STUDENT.to_string(),
            "00000000-0000-0000-0000-000000000004"
        );
    }

    #[test]
    fn test_is_system_role() {
        assert!(system_roles::is_system_role(&system_roles::SYSTEM_ADMIN));
        assert!(system_roles::is_system_role(&system_roles::ADMIN));
        assert!(system_roles::is_system_role(&system_roles::TEACHER));
        assert!(system_roles::is_system_role(&system_roles::STUDENT));
        assert!(!system_roles::is_system_role(&Uuid::new_v4()));
    }

    #[test]
    fn test_get_user_id_from_auth() {
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

        let result = get_user_id_from_auth(&auth_user);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), user_id);
    }

    #[test]
    fn test_get_user_id_from_auth_invalid() {
        let claims = Claims {
            sub: "not-a-uuid".to_string(),
            email: "test@example.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        let result = get_user_id_from_auth(&auth_user);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_system_admin_jwt() {
        let auth_user = create_test_auth_user(vec![system_roles::SYSTEM_ADMIN], vec![]);
        assert!(is_system_admin_jwt(&auth_user));

        let non_admin = create_test_auth_user(vec![system_roles::TEACHER], vec![]);
        assert!(!is_system_admin_jwt(&non_admin));
    }

    #[test]
    fn test_is_admin_jwt() {
        let sys_admin = create_test_auth_user(vec![system_roles::SYSTEM_ADMIN], vec![]);
        assert!(is_admin_jwt(&sys_admin));

        let admin = create_test_auth_user(vec![system_roles::ADMIN], vec![]);
        assert!(is_admin_jwt(&admin));

        let teacher = create_test_auth_user(vec![system_roles::TEACHER], vec![]);
        assert!(!is_admin_jwt(&teacher));
    }

    #[test]
    fn test_is_teacher_or_above_jwt() {
        let sys_admin = create_test_auth_user(vec![system_roles::SYSTEM_ADMIN], vec![]);
        assert!(is_teacher_or_above_jwt(&sys_admin));

        let admin = create_test_auth_user(vec![system_roles::ADMIN], vec![]);
        assert!(is_teacher_or_above_jwt(&admin));

        let teacher = create_test_auth_user(vec![system_roles::TEACHER], vec![]);
        assert!(is_teacher_or_above_jwt(&teacher));

        let student = create_test_auth_user(vec![system_roles::STUDENT], vec![]);
        assert!(!is_teacher_or_above_jwt(&student));
    }

    #[test]
    fn test_check_user_has_permission_jwt() {
        let auth_user = create_test_auth_user(
            vec![],
            vec!["users:read".to_string(), "users:create".to_string()],
        );

        assert!(check_user_has_permission_jwt(&auth_user, "users:read"));
        assert!(check_user_has_permission_jwt(&auth_user, "users:create"));
        assert!(!check_user_has_permission_jwt(&auth_user, "users:delete"));
    }

    #[test]
    fn test_check_user_has_any_role_jwt() {
        let auth_user = create_test_auth_user(vec![system_roles::ADMIN], vec![]);

        assert!(check_user_has_any_role_jwt(
            &auth_user,
            &[system_roles::SYSTEM_ADMIN, system_roles::ADMIN]
        ));
        assert!(!check_user_has_any_role_jwt(
            &auth_user,
            &[system_roles::TEACHER, system_roles::STUDENT]
        ));
    }

    #[test]
    fn test_get_school_id_from_auth() {
        let school_id = Uuid::new_v4();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            school_id: Some(school_id),
            role_ids: vec![system_roles::ADMIN],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(get_school_id_from_auth(&auth_user), Some(school_id));
    }

    #[test]
    fn test_get_school_id_from_auth_system_admin() {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "sysadmin@example.com".to_string(),
            school_id: None,
            role_ids: vec![system_roles::SYSTEM_ADMIN],
            permissions: vec![],
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        assert_eq!(get_school_id_from_auth(&auth_user), None);
    }
}
