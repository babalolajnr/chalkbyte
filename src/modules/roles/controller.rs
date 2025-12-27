use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::middleware::role::{get_user_id_from_auth, is_admin, is_system_admin};
use crate::modules::users::model::system_roles;
use crate::state::AppState;
use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;

use super::model::{
    AssignPermissionsDto, AssignRoleToUserDto, CreateCustomRoleDto, CustomRoleWithPermissions,
    PaginatedPermissionsResponse, PaginatedRolesResponse, Permission, PermissionFilterParams,
    RoleAssignmentResponse, RoleFilterParams, UpdateCustomRoleDto,
};
use super::service;

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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    // Get requester's school_id
    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    let role = service::create_custom_role(
        &state.db,
        dto,
        requester.school_id,
        user_is_system_admin,
        user_id,
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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    let result =
        service::get_custom_roles(&state.db, params, requester.school_id, user_is_system_admin)
            .await?;

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role details with permissions", body = CustomRoleWithPermissions),
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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    let role =
        service::get_custom_role_by_id(&state.db, id, requester.school_id, user_is_system_admin)
            .await?;

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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    // Cannot update system roles unless you're a system admin
    let role =
        service::get_custom_role_by_id(&state.db, id, requester.school_id, user_is_system_admin)
            .await?;
    if role.role.is_system_role && !user_is_system_admin {
        return Err(AppError::forbidden(
            "Only system admins can update system roles".to_string(),
        ));
    }

    let updated_role = service::update_custom_role(
        &state.db,
        id,
        dto,
        requester.school_id,
        user_is_system_admin,
    )
    .await?;

    Ok(Json(updated_role))
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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    // Cannot delete system roles
    if system_roles::is_system_role(&id) {
        return Err(AppError::forbidden(
            "Cannot delete built-in system roles".to_string(),
        ));
    }

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    service::delete_custom_role(&state.db, id, requester.school_id, user_is_system_admin).await?;

    Ok(())
}

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
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignPermissionsDto>,
) -> Result<Json<CustomRoleWithPermissions>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    // Check if user can modify this role
    let role =
        service::get_custom_role_by_id(&state.db, id, requester.school_id, user_is_system_admin)
            .await?;
    if role.role.is_system_role && !user_is_system_admin {
        return Err(AppError::forbidden(
            "Only system admins can modify system role permissions".to_string(),
        ));
    }

    let updated_role = service::assign_permissions_to_role(
        &state.db,
        id,
        dto.permission_ids,
        requester.school_id,
        user_is_system_admin,
    )
    .await?;

    Ok(Json(updated_role))
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
    let user_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, user_id).await?;

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;

    // Check if user can modify this role
    let role = service::get_custom_role_by_id(
        &state.db,
        role_id,
        requester.school_id,
        user_is_system_admin,
    )
    .await?;
    if role.role.is_system_role && !user_is_system_admin {
        return Err(AppError::forbidden(
            "Only system admins can modify system role permissions".to_string(),
        ));
    }

    let updated_role = service::remove_permission_from_role(
        &state.db,
        role_id,
        permission_id,
        requester.school_id,
        user_is_system_admin,
    )
    .await?;

    Ok(Json(updated_role))
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
        (status = 200, description = "Role assigned to user", body = RoleAssignmentResponse),
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
    Path(target_user_id): Path<Uuid>,
    Json(dto): Json<AssignRoleToUserDto>,
) -> Result<Json<RoleAssignmentResponse>, AppError> {
    let requester_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, requester_id).await?;
    let user_is_admin = is_admin(&state.db, requester_id).await?;

    // Only admins can assign roles
    if !user_is_admin {
        return Err(AppError::forbidden(
            "Only admins can assign roles to users".to_string(),
        ));
    }

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
        .fetch_one(&state.db)
        .await?;

    // Check if trying to assign a system role
    if system_roles::is_system_role(&dto.role_id) && !user_is_system_admin {
        return Err(AppError::forbidden(
            "Only system admins can assign system roles".to_string(),
        ));
    }

    // Verify target user is in the same school (unless system admin)
    if !user_is_system_admin {
        let target_user = sqlx::query!("SELECT school_id FROM users WHERE id = $1", target_user_id)
            .fetch_one(&state.db)
            .await?;

        if target_user.school_id != requester.school_id {
            return Err(AppError::forbidden(
                "You can only assign roles to users in your school".to_string(),
            ));
        }
    }

    let response = service::assign_role_to_user(
        &state.db,
        target_user_id,
        dto.role_id,
        requester_id,
        requester.school_id,
        user_is_system_admin,
    )
    .await?;

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/{user_id}/roles/{role_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role removed from user"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User or role assignment not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn remove_role_from_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((target_user_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    let requester_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, requester_id).await?;
    let user_is_admin = is_admin(&state.db, requester_id).await?;

    // Only admins can remove roles
    if !user_is_admin {
        return Err(AppError::forbidden(
            "Only admins can remove roles from users".to_string(),
        ));
    }

    let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
        .fetch_one(&state.db)
        .await?;

    // Check if trying to remove a system role
    if system_roles::is_system_role(&role_id) && !user_is_system_admin {
        return Err(AppError::forbidden(
            "Only system admins can remove system roles".to_string(),
        ));
    }

    service::remove_role_from_user(
        &state.db,
        target_user_id,
        role_id,
        requester.school_id,
        user_is_system_admin,
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
    Path(target_user_id): Path<Uuid>,
) -> Result<Json<Vec<CustomRoleWithPermissions>>, AppError> {
    let requester_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, requester_id).await?;
    let user_is_admin = is_admin(&state.db, requester_id).await?;

    // Users can view their own roles, admins can view any user's roles in their school
    if requester_id != target_user_id && !user_is_admin {
        return Err(AppError::forbidden(
            "You can only view your own roles".to_string(),
        ));
    }

    // If not system admin, verify target is in same school
    if !user_is_system_admin && requester_id != target_user_id {
        let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
            .fetch_one(&state.db)
            .await?;

        let target_user = sqlx::query!("SELECT school_id FROM users WHERE id = $1", target_user_id)
            .fetch_one(&state.db)
            .await?;

        if target_user.school_id != requester.school_id {
            return Err(AppError::forbidden(
                "You can only view roles for users in your school".to_string(),
            ));
        }
    }

    let roles = service::get_user_roles(&state.db, target_user_id).await?;

    Ok(Json(roles))
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/permissions",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User's permissions from all roles", body = Vec<Permission>),
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
    Path(target_user_id): Path<Uuid>,
) -> Result<Json<Vec<Permission>>, AppError> {
    let requester_id = get_user_id_from_auth(&auth_user)?;
    let user_is_system_admin = is_system_admin(&state.db, requester_id).await?;
    let user_is_admin = is_admin(&state.db, requester_id).await?;

    // Users can view their own permissions, admins can view any user's permissions
    if requester_id != target_user_id && !user_is_admin {
        return Err(AppError::forbidden(
            "You can only view your own permissions".to_string(),
        ));
    }

    // If not system admin, verify target is in same school
    if !user_is_system_admin && requester_id != target_user_id {
        let requester = sqlx::query!("SELECT school_id FROM users WHERE id = $1", requester_id)
            .fetch_one(&state.db)
            .await?;

        let target_user = sqlx::query!("SELECT school_id FROM users WHERE id = $1", target_user_id)
            .fetch_one(&state.db)
            .await?;

        if target_user.school_id != requester.school_id {
            return Err(AppError::forbidden(
                "You can only view permissions for users in your school".to_string(),
            ));
        }
    }

    let permissions = service::get_user_permissions(&state.db, target_user_id).await?;

    Ok(Json(permissions))
}
