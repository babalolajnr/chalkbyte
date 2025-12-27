//! Role-based authorization middleware for Axum
//!
//! This module provides permission-based access control using the database
//! to check user roles and permissions dynamically.

#![allow(dead_code)]

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
use crate::modules::users::service::UserService;
use crate::state::AppState;
use crate::utils::errors::AppError;

/// Middleware function that checks if the authenticated user has one of the required roles.
pub async fn require_roles(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    allowed_role_ids: Vec<Uuid>,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(user) => user,
        Err(e) => return Err(e),
    };

    let user_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))?;

    // Check if user has any of the allowed roles
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
pub async fn require_permission(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
    permission_name: &str,
) -> Result<Response, AppError> {
    let (mut parts, body) = req.into_parts();

    let auth_user = match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(user) => user,
        Err(e) => return Err(e),
    };

    let user_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))?;

    // Check if user has the permission
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

/// Helper function to create a middleware closure for system admin only routes
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

/// Simple role checker extractor for system admin requirement
#[derive(Debug, Clone)]
pub struct RequireSystemAdmin(pub Uuid);

impl FromRequestParts<AppState> for RequireSystemAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))?;

        let is_sys_admin = UserService::is_system_admin(&state.db, user_id).await?;

        if !is_sys_admin {
            return Err(AppError::forbidden(
                "Access denied. Only system administrators can access this resource.".to_string(),
            ));
        }

        Ok(RequireSystemAdmin(user_id))
    }
}

/// Extractor for admin-level access (SystemAdmin or Admin)
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub Uuid);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))?;

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

/// Extractor for teacher-level access (SystemAdmin, Admin, or Teacher)
#[derive(Debug, Clone)]
pub struct RequireTeacher(pub Uuid);

impl FromRequestParts<AppState> for RequireTeacher {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let user_id = Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))?;

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

/// Extractor that checks for a specific permission
#[derive(Debug, Clone)]
pub struct RequirePermission<const N: usize>(pub Uuid);

/// Helper function to check if a user has any of the specified roles (async version)
pub async fn check_user_has_any_role(
    db: &sqlx::PgPool,
    user_id: Uuid,
    role_ids: &[Uuid],
) -> Result<bool, AppError> {
    UserService::user_has_any_role(db, user_id, role_ids).await
}

/// Helper function to check if a user has a specific permission (async version)
pub async fn check_user_has_permission(
    db: &sqlx::PgPool,
    user_id: Uuid,
    permission_name: &str,
) -> Result<bool, AppError> {
    roles_service::user_has_permission(db, user_id, permission_name).await
}

/// Check if user is a system admin
pub async fn is_system_admin(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    UserService::is_system_admin(db, user_id).await
}

/// Check if user is an admin (school admin or system admin)
pub async fn is_admin(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    UserService::user_has_any_role(
        db,
        user_id,
        &[system_roles::SYSTEM_ADMIN, system_roles::ADMIN],
    )
    .await
}

/// Check if user is at least a teacher (teacher, admin, or system admin)
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

/// Get the user ID from auth claims
pub fn get_user_id_from_auth(auth_user: &AuthUser) -> Result<Uuid, AppError> {
    Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID in token".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use crate::modules::auth::model::Claims;

        let user_id = Uuid::new_v4();
        let claims = Claims {
            sub: user_id.to_string(),
            email: "test@example.com".to_string(),
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
        use crate::modules::auth::model::Claims;

        let claims = Claims {
            sub: "not-a-uuid".to_string(),
            email: "test@example.com".to_string(),
            exp: 9999999999,
            iat: 1234567890,
        };
        let auth_user = AuthUser(claims);

        let result = get_user_id_from_auth(&auth_user);
        assert!(result.is_err());
    }
}
