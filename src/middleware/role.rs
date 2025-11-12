//! Role-based authorization middleware for Axum
//!
//! This module provides multiple approaches for role-based access control:
//! 1. Layer-based middleware using `RequireRole`
//! 2. Extractor-based approach using `RequireRoles`
//! 3. Helper functions for manual role checking

#![allow(dead_code)]

use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::users::model::UserRole;
use crate::utils::errors::AppError;

/// Middleware function that checks if the authenticated user has one of the required roles.
///
/// # Usage with axum::middleware::from_fn_with_state
///
/// ```rust,ignore
/// use axum::{Router, middleware};
/// use crate::middleware::role::require_roles;
/// use crate::modules::users::model::UserRole;
///
/// let protected_routes = Router::new()
///     .route("/admin-only", get(admin_handler))
///     .layer(middleware::from_fn_with_state(
///         state.clone(),
///         |state, req, next| require_roles(state, req, next, vec![UserRole::SystemAdmin])
///     ));
/// ```
pub async fn require_roles(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    allowed_roles: Vec<UserRole>,
) -> Result<Response, AppError> {
    // Extract the authenticated user from request parts
    let (mut parts, body) = req.into_parts();

    let auth_user = match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(user) => user,
        Err(e) => return Err(e),
    };

    // Parse the user's role
    let user_role = parse_role_from_string(&auth_user.0.role)?;

    // Check if user has one of the allowed roles
    if !allowed_roles.contains(&user_role) {
        return Err(AppError::forbidden(format!(
            "Access denied. Required roles: {:?}, but user has role: {:?}",
            allowed_roles, user_role
        )));
    }

    // Reconstruct the request and continue
    req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Helper function to create a middleware closure for system admin only routes
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, middleware};
/// use crate::middleware::role::require_system_admin;
///
/// let admin_routes = Router::new()
///     .route("/system-settings", get(settings_handler))
///     .layer(middleware::from_fn_with_state(state.clone(), require_system_admin));
/// ```
pub async fn require_system_admin(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    match require_roles(State(state), req, next, vec![UserRole::SystemAdmin]).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Helper function for school admin routes (both SystemAdmin and Admin allowed)
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, middleware};
/// use crate::middleware::role::require_admin;
///
/// let admin_routes = Router::new()
///     .route("/school-settings", get(settings_handler))
///     .layer(middleware::from_fn_with_state(state.clone(), require_admin));
/// ```
pub async fn require_admin(State(state): State<AppState>, req: Request, next: Next) -> Response {
    match require_roles(
        State(state),
        req,
        next,
        vec![UserRole::SystemAdmin, UserRole::Admin],
    )
    .await
    {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Helper function for teacher routes (SystemAdmin, Admin, and Teacher allowed)
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, middleware};
/// use crate::middleware::role::require_teacher;
///
/// let teacher_routes = Router::new()
///     .route("/grades", get(grades_handler))
///     .layer(middleware::from_fn_with_state(state.clone(), require_teacher));
/// ```
pub async fn require_teacher(State(state): State<AppState>, req: Request, next: Next) -> Response {
    match require_roles(
        State(state),
        req,
        next,
        vec![UserRole::SystemAdmin, UserRole::Admin, UserRole::Teacher],
    )
    .await
    {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Simple role checker extractor for single role requirement
///
/// # Example
///
/// ```rust,ignore
/// use crate::middleware::role::RequireSystemAdmin;
///
/// pub async fn system_handler(
///     _require_admin: RequireSystemAdmin,
///     auth_user: AuthUser,
/// ) -> Result<Json<Response>, AppError> {
///     // Only system admins can access this
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RequireSystemAdmin;

impl FromRequestParts<AppState> for RequireSystemAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_role = parse_role_from_string(&auth_user.0.role)?;

        if user_role != UserRole::SystemAdmin {
            return Err(AppError::forbidden(
                "Access denied. Only system administrators can access this resource.".to_string(),
            ));
        }

        Ok(RequireSystemAdmin)
    }
}

/// Extractor for admin-level access (SystemAdmin or Admin)
#[derive(Debug, Clone)]
pub struct RequireAdmin;

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_role = parse_role_from_string(&auth_user.0.role)?;

        if user_role != UserRole::SystemAdmin && user_role != UserRole::Admin {
            return Err(AppError::forbidden(
                "Access denied. Administrator privileges required.".to_string(),
            ));
        }

        Ok(RequireAdmin)
    }
}

/// Extractor for teacher-level access (SystemAdmin, Admin, or Teacher)
#[derive(Debug, Clone)]
pub struct RequireTeacher;

impl FromRequestParts<AppState> for RequireTeacher {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_role = parse_role_from_string(&auth_user.0.role)?;

        if user_role != UserRole::SystemAdmin
            && user_role != UserRole::Admin
            && user_role != UserRole::Teacher
        {
            return Err(AppError::forbidden(
                "Access denied. Teacher privileges required.".to_string(),
            ));
        }

        Ok(RequireTeacher)
    }
}

/// Helper function to check if a user has a specific role in controller logic
///
/// # Example
///
/// ```rust,ignore
/// use crate::middleware::role::check_role;
/// use crate::modules::users::model::UserRole;
///
/// pub async fn handler(auth_user: AuthUser) -> Result<Json<Response>, AppError> {
///     check_role(&auth_user, UserRole::SystemAdmin)?;
///     // Handler logic
/// }
/// ```
pub fn check_role(auth_user: &AuthUser, required_role: UserRole) -> Result<(), AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;

    if user_role != required_role {
        return Err(AppError::forbidden(format!(
            "Access denied. Required role: {:?}, but user has role: {:?}",
            required_role, user_role
        )));
    }

    Ok(())
}

/// Helper function to check if a user has any of the specified roles
///
/// # Example
///
/// ```rust,ignore
/// use crate::middleware::role::check_any_role;
/// use crate::modules::users::model::UserRole;
///
/// pub async fn handler(auth_user: AuthUser) -> Result<Json<Response>, AppError> {
///     check_any_role(&auth_user, &[UserRole::SystemAdmin, UserRole::Admin])?;
///     // Handler logic
/// }
/// ```
pub fn check_any_role(auth_user: &AuthUser, allowed_roles: &[UserRole]) -> Result<(), AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;

    if !allowed_roles.contains(&user_role) {
        return Err(AppError::forbidden(format!(
            "Access denied. Required roles: {:?}, but user has role: {:?}",
            allowed_roles, user_role
        )));
    }

    Ok(())
}

/// Parse a role string into a UserRole enum
fn parse_role_from_string(role_str: &str) -> Result<UserRole, AppError> {
    match role_str {
        "system_admin" => Ok(UserRole::SystemAdmin),
        "admin" => Ok(UserRole::Admin),
        "teacher" => Ok(UserRole::Teacher),
        "student" => Ok(UserRole::Student),
        _ => Err(AppError::internal_error(format!(
            "Invalid role: {}",
            role_str
        ))),
    }
}

/// Get the hierarchy level of a role (higher number = more privileges)
pub fn role_hierarchy_level(role: &UserRole) -> u8 {
    match role {
        UserRole::SystemAdmin => 3,
        UserRole::Admin => 2,
        UserRole::Teacher => 1,
        UserRole::Student => 0,
    }
}

/// Check if a role has at least the specified level of access
///
/// # Example
///
/// ```rust,ignore
/// use crate::middleware::role::{check_role_hierarchy, parse_role_from_string};
/// use crate::modules::users::model::UserRole;
///
/// let user_role = parse_role_from_string(&auth_user.0.role)?;
/// check_role_hierarchy(&user_role, &UserRole::Admin)?;
/// ```
pub fn check_role_hierarchy(
    user_role: &UserRole,
    minimum_required_role: &UserRole,
) -> Result<(), AppError> {
    if role_hierarchy_level(user_role) < role_hierarchy_level(minimum_required_role) {
        return Err(AppError::forbidden(format!(
            "Access denied. Minimum required role: {:?}, but user has role: {:?}",
            minimum_required_role, user_role
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_hierarchy() {
        assert_eq!(role_hierarchy_level(&UserRole::SystemAdmin), 3);
        assert_eq!(role_hierarchy_level(&UserRole::Admin), 2);
        assert_eq!(role_hierarchy_level(&UserRole::Teacher), 1);
        assert_eq!(role_hierarchy_level(&UserRole::Student), 0);
    }

    #[test]
    fn test_parse_role_from_string() {
        assert!(matches!(
            parse_role_from_string("system_admin"),
            Ok(UserRole::SystemAdmin)
        ));
        assert!(matches!(
            parse_role_from_string("admin"),
            Ok(UserRole::Admin)
        ));
        assert!(matches!(
            parse_role_from_string("teacher"),
            Ok(UserRole::Teacher)
        ));
        assert!(matches!(
            parse_role_from_string("student"),
            Ok(UserRole::Student)
        ));
        assert!(parse_role_from_string("invalid").is_err());
    }
}
