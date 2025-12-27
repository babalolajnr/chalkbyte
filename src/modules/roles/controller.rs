use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::modules::users::model::UserRole;
use crate::state::AppState;
use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;

use super::model::{
    AssignPermissionsDto, AssignRoleToUserDto, CreateCustomRoleDto, CustomRoleWithPermissions,
    PaginatedPermissionsResponse, PaginatedRolesResponse, Permission, PermissionFilterParams,
    RoleAssignmentResponse, RoleFilterParams, UpdateCustomRoleDto,
};
use super::service;

fn parse_user_role(role_str: &str) -> Result<UserRole, AppError> {
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

// ============ Permission Endpoints ============

#[utoipa::path(
    get,
    path = "/api/roles/permissions",
    params(
        ("category" = Option<String>, Query, description = "Filter by permission category"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of permissions", body = PaginatedPermissionsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_permissions(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(params): Query<PermissionFilterParams>,
) -> Result<Json<PaginatedPermissionsResponse>, AppError> {
    let result = service::get_all_permissions(&state.db, params).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/roles/permissions/{id}",
    params(
        ("id" = Uuid, Path, description = "Permission ID")
    ),
    responses(
        (status = 200, description = "Permission details", body = Permission),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Permission not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_permission_by_id(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Permission>, AppError> {
    let permission = service::get_permission_by_id(&state.db, id).await?;
    Ok(Json(permission))
}

// ============ Custom Role Endpoints ============

#[utoipa::path(
    post,
    path = "/api/roles",
    request_body = CreateCustomRoleDto,
    responses(
        (status = 201, description = "Role created successfully", body = CustomRoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn create_role(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(dto): ValidatedJson<CreateCustomRoleDto>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    // Get requester's school_id
    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let role = service::create_custom_role(
        &state.db,
        dto,
        requester.school_id,
        is_system_admin,
        Uuid::parse_str(&auth_user.0.sub).unwrap(),
    )
    .await?;

    Ok(Json(role))
}

#[utoipa::path(
    get,
    path = "/api/roles",
    params(
        ("school_id" = Option<Uuid>, Query, description = "Filter by school ID"),
        ("is_system_role" = Option<bool>, Query, description = "Filter system roles only"),
        ("name" = Option<String>, Query, description = "Search by name"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of roles", body = PaginatedRolesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_roles(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<RoleFilterParams>,
) -> Result<Json<PaginatedRolesResponse>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let result =
        service::get_custom_roles(&state.db, params, requester.school_id, is_system_admin).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role details", body = CustomRoleWithPermissions),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_role_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let role =
        service::get_custom_role_by_id(&state.db, id, requester.school_id, is_system_admin).await?;
    Ok(Json(role))
}

#[utoipa::path(
    put,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    request_body = UpdateCustomRoleDto,
    responses(
        (status = 200, description = "Role updated successfully", body = CustomRoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn update_role(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpdateCustomRoleDto>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let role =
        service::update_custom_role(&state.db, id, dto, requester.school_id, is_system_admin)
            .await?;
    Ok(Json(role))
}

#[utoipa::path(
    delete,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    service::delete_custom_role(&state.db, id, requester.school_id, is_system_admin).await?;
    Ok(())
}

// ============ Role Permissions Endpoints ============

#[utoipa::path(
    post,
    path = "/api/roles/{id}/permissions",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    request_body = AssignPermissionsDto,
    responses(
        (status = 200, description = "Permissions assigned successfully", body = CustomRoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn assign_permissions(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(role_id): Path<Uuid>,
    Json(dto): Json<AssignPermissionsDto>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let role = service::assign_permissions_to_role(
        &state.db,
        role_id,
        dto,
        requester.school_id,
        is_system_admin,
    )
    .await?;
    Ok(Json(role))
}

#[utoipa::path(
    delete,
    path = "/api/roles/{role_id}/permissions/{permission_id}",
    params(
        ("role_id" = Uuid, Path, description = "Role ID"),
        ("permission_id" = Uuid, Path, description = "Permission ID")
    ),
    responses(
        (status = 200, description = "Permission removed successfully", body = CustomRoleWithPermissions),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Role or permission not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn remove_permission(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let role = service::remove_permission_from_role(
        &state.db,
        role_id,
        permission_id,
        requester.school_id,
        is_system_admin,
    )
    .await?;
    Ok(Json(role))
}

// ============ User Role Assignment Endpoints ============

#[utoipa::path(
    post,
    path = "/api/users/{user_id}/roles",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = AssignRoleToUserDto,
    responses(
        (status = 200, description = "Role assigned successfully", body = RoleAssignmentResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User or role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn assign_role_to_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(dto): Json<AssignRoleToUserDto>,
) -> Result<Json<RoleAssignmentResponse>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
        .fetch_one(&state.db)
        .await?;

    let assignment = service::assign_role_to_user(
        &state.db,
        user_id,
        dto.role_id,
        requester_id,
        requester.school_id,
        is_system_admin,
    )
    .await?;

    Ok(Json(RoleAssignmentResponse {
        message: "Role assigned successfully".to_string(),
        user_id: assignment.user_id,
        role_id: assignment.role_id,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/users/{user_id}/roles/{role_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role removed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User or role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn remove_role_from_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    service::remove_role_from_user(
        &state.db,
        user_id,
        role_id,
        requester.school_id,
        is_system_admin,
    )
    .await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/roles",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User's roles", body = Vec<CustomRoleWithPermissions>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_user_roles(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<CustomRoleWithPermissions>>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    let requester = sqlx::query!(
        "SELECT school_id FROM users WHERE id = $1",
        Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?
    )
    .fetch_one(&state.db)
    .await?;

    let roles =
        service::get_user_roles(&state.db, user_id, requester.school_id, is_system_admin).await?;
    Ok(Json(roles))
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/permissions",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User's permissions", body = Vec<Permission>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_user_permissions(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<Permission>>, AppError> {
    let user_role = parse_user_role(&auth_user.0.role)?;
    let is_system_admin = user_role == UserRole::SystemAdmin;

    // Authorization check: user can view their own permissions, or admin can view any
    let requester_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::internal_error("Invalid user ID".to_string()))?;

    if requester_id != user_id && !is_system_admin && user_role != UserRole::Admin {
        return Err(AppError::forbidden(
            "You can only view your own permissions".to_string(),
        ));
    }

    // For school admins, verify user is in their school
    if user_role == UserRole::Admin {
        let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
            .fetch_one(&state.db)
            .await?;
        let target_user = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
            .fetch_one(&state.db)
            .await?;

        if requester.school_id != target_user.school_id {
            return Err(AppError::forbidden(
                "You can only view permissions for users in your school".to_string(),
            ));
        }
    }

    let permissions = service::get_user_permissions(&state.db, user_id).await?;
    Ok(Json(permissions))
}
