use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use chalkbyte_core::AppError;
use chalkbyte_models::ids::{PermissionId, RoleId, SchoolId, UserId};

use crate::middleware::auth::{
    RequireRolesAssign, RequireRolesCreate, RequireRolesDelete, RequireRolesRead,
    RequireRolesUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::users::model::system_roles;
use crate::state::AppState;
use crate::utils::auth_helpers::get_admin_school_id;
use crate::validator::ValidatedJson;

use super::model::{
    AssignPermissionsDto, AssignRoleToUserDto, CreateRoleDto, PaginatedPermissionsResponse,
    PaginatedRolesResponse, Permission, PermissionFilterParams, RoleAssignmentResponse,
    RoleFilterParams, RoleWithPermissions, UpdateRoleDto,
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
        (status = 403, description = "Forbidden - requires roles:read permission")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_permissions(
    State(state): State<AppState>,
    RequireRolesRead(auth_user): RequireRolesRead,
    Query(params): Query<PermissionFilterParams>,
) -> Result<Json<PaginatedPermissionsResponse>, AppError> {
    let is_sys_admin = is_system_admin_jwt(&auth_user);
    let result = service::get_all_permissions(&state.db, params, is_sys_admin).await?;
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
        (status = 403, description = "Forbidden - requires roles:read permission"),
        (status = 404, description = "Permission not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_permission_by_id(
    State(state): State<AppState>,
    RequireRolesRead(_auth_user): RequireRolesRead,
    Path(id): Path<Uuid>,
) -> Result<Json<Permission>, AppError> {
    let permission_id = PermissionId::from(id);
    let permission = service::get_permission_by_id(&state.db, permission_id).await?;
    Ok(Json(permission))
}

// ============ Role Endpoints ============

#[utoipa::path(
    post,
    path = "/api/roles",
    request_body = CreateRoleDto,
    responses(
        (status = 201, description = "Role created successfully", body = RoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:create permission")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn create_role(
    State(state): State<AppState>,
    RequireRolesCreate(auth_user): RequireRolesCreate,
    ValidatedJson(dto): ValidatedJson<CreateRoleDto>,
) -> Result<Json<RoleWithPermissions>, AppError> {
    let user_id = auth_user.user_id()?;
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    let role = service::create_role(
        &state.db,
        state.cache.as_ref(),
        dto,
        school_id,
        is_sys_admin,
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
        (status = 403, description = "Forbidden - requires roles:read permission")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_roles(
    State(state): State<AppState>,
    RequireRolesRead(auth_user): RequireRolesRead,
    Query(params): Query<RoleFilterParams>,
) -> Result<Json<PaginatedRolesResponse>, AppError> {
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    let result = service::get_roles(&state.db, params, school_id, is_sys_admin).await?;

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role details with permissions", body = RoleWithPermissions),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:read permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_role_by_id(
    State(state): State<AppState>,
    RequireRolesRead(auth_user): RequireRolesRead,
    Path(id): Path<Uuid>,
) -> Result<Json<RoleWithPermissions>, AppError> {
    let role_id = RoleId::from(id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    let role = service::get_role_by_id(&state.db, role_id, school_id, is_sys_admin).await?;

    Ok(Json(role))
}

#[utoipa::path(
    put,
    path = "/api/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "Role ID")
    ),
    request_body = UpdateRoleDto,
    responses(
        (status = 200, description = "Role updated successfully", body = RoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:update permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn update_role(
    State(state): State<AppState>,
    RequireRolesUpdate(auth_user): RequireRolesUpdate,
    Path(id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpdateRoleDto>,
) -> Result<Json<RoleWithPermissions>, AppError> {
    let role_id = RoleId::from(id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    // Cannot update system roles unless you're a system admin
    let role = service::get_role_by_id(&state.db, role_id, school_id, is_sys_admin).await?;
    if role.role.is_system_role && !is_sys_admin {
        return Err(AppError::forbidden(
            "Only system admins can update system roles".to_string(),
        ));
    }

    let updated_role = service::update_role(
        &state.db,
        state.cache.as_ref(),
        role_id,
        dto,
        school_id,
        is_sys_admin,
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
        (status = 403, description = "Forbidden - requires roles:delete permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    RequireRolesDelete(auth_user): RequireRolesDelete,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let role_id = RoleId::from(id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    // Cannot delete system roles
    if system_roles::is_system_role(&role_id) {
        return Err(AppError::forbidden(
            "Cannot delete built-in system roles".to_string(),
        ));
    }

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    service::delete_role(
        &state.db,
        state.cache.as_ref(),
        role_id,
        school_id,
        is_sys_admin,
    )
    .await?;

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
        (status = 200, description = "Permissions assigned successfully", body = RoleWithPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:update permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn assign_permissions(
    State(state): State<AppState>,
    RequireRolesUpdate(auth_user): RequireRolesUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignPermissionsDto>,
) -> Result<Json<RoleWithPermissions>, AppError> {
    let role_id = RoleId::from(id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    // Check if user can modify this role
    let role = service::get_role_by_id(&state.db, role_id, school_id, is_sys_admin).await?;
    if role.role.is_system_role && !is_sys_admin {
        return Err(AppError::forbidden(
            "Only system admins can modify system role permissions".to_string(),
        ));
    }

    let updated_role = service::assign_permissions_to_role(
        &state.db,
        state.cache.as_ref(),
        role_id,
        &dto.permission_ids,
        school_id,
        is_sys_admin,
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
        (status = 200, description = "Permission removed successfully", body = RoleWithPermissions),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:update permission"),
        (status = 404, description = "Role or permission not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn remove_permission(
    State(state): State<AppState>,
    RequireRolesUpdate(auth_user): RequireRolesUpdate,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RoleWithPermissions>, AppError> {
    let role_id = RoleId::from(role_id);
    let permission_id = PermissionId::from(permission_id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    // Check if user can modify this role
    let role = service::get_role_by_id(&state.db, role_id, school_id, is_sys_admin).await?;
    if role.role.is_system_role && !is_sys_admin {
        return Err(AppError::forbidden(
            "Only system admins can modify system role permissions".to_string(),
        ));
    }

    let updated_role = service::remove_permission_from_role(
        &state.db,
        state.cache.as_ref(),
        role_id,
        permission_id,
        school_id,
        is_sys_admin,
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
        (status = 403, description = "Forbidden - requires roles:assign permission"),
        (status = 404, description = "User or role not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn assign_role_to_user(
    State(state): State<AppState>,
    RequireRolesAssign(auth_user): RequireRolesAssign,
    Path(target_user_id): Path<Uuid>,
    Json(dto): Json<AssignRoleToUserDto>,
) -> Result<Json<RoleAssignmentResponse>, AppError> {
    let target_user_id = UserId::from(target_user_id);
    let requester_id = auth_user.user_id()?;
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    // Check if trying to assign a system role
    if system_roles::is_system_role(&dto.role_id) && !is_sys_admin {
        return Err(AppError::forbidden(
            "Only system admins can assign system roles".to_string(),
        ));
    }

    // Verify target user is in the same school (unless system admin)
    if !is_sys_admin {
        let target_user = sqlx::query!(
            "SELECT school_id FROM users WHERE id = $1",
            target_user_id.into_inner()
        )
        .fetch_one(&state.db)
        .await?;

        let target_school_id = target_user.school_id.map(SchoolId::from);
        if target_school_id != school_id {
            return Err(AppError::forbidden(
                "You can only assign roles to users in your school".to_string(),
            ));
        }
    }

    let response = service::assign_role_to_user(
        &state.db,
        state.cache.as_ref(),
        target_user_id,
        dto.role_id,
        requester_id,
        school_id,
        is_sys_admin,
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
        (status = 403, description = "Forbidden - requires roles:assign permission"),
        (status = 404, description = "User or role assignment not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn remove_role_from_user(
    State(state): State<AppState>,
    RequireRolesAssign(auth_user): RequireRolesAssign,
    Path((target_user_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    let target_user_id = UserId::from(target_user_id);
    let role_id = RoleId::from(role_id);
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    let school_id = if is_sys_admin {
        None
    } else {
        Some(get_admin_school_id(&state.db, &auth_user).await?)
    };

    // Check if trying to remove a system role
    if system_roles::is_system_role(&role_id) && !is_sys_admin {
        return Err(AppError::forbidden(
            "Only system admins can remove system roles".to_string(),
        ));
    }

    service::remove_role_from_user(
        &state.db,
        state.cache.as_ref(),
        target_user_id,
        role_id,
        school_id,
        is_sys_admin,
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
        (status = 200, description = "User's roles", body = Vec<RoleWithPermissions>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires roles:read permission"),
        (status = 404, description = "User not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_user_roles(
    State(state): State<AppState>,
    RequireRolesRead(auth_user): RequireRolesRead,
    Path(target_user_id): Path<Uuid>,
) -> Result<Json<Vec<RoleWithPermissions>>, AppError> {
    let target_user_id = UserId::from(target_user_id);
    let requester_id = auth_user.user_id()?;
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    // Users can view their own roles without additional checks
    if requester_id != target_user_id && !is_sys_admin {
        let school_id = get_admin_school_id(&state.db, &auth_user).await?;

        let target_user = sqlx::query!(
            "SELECT school_id FROM users WHERE id = $1",
            target_user_id.into_inner()
        )
        .fetch_one(&state.db)
        .await?;

        let target_school_id = target_user.school_id.map(SchoolId::from);
        if target_school_id != Some(school_id) {
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
        (status = 403, description = "Forbidden - requires roles:read permission"),
        (status = 404, description = "User not found")
    ),
    tag = "Roles",
    security(("bearer_auth" = []))
)]
pub async fn get_user_permissions(
    State(state): State<AppState>,
    RequireRolesRead(auth_user): RequireRolesRead,
    Path(target_user_id): Path<Uuid>,
) -> Result<Json<Vec<Permission>>, AppError> {
    let target_user_id = UserId::from(target_user_id);
    let requester_id = auth_user.user_id()?;
    let is_sys_admin = is_system_admin_jwt(&auth_user);

    // Users can view their own permissions without additional checks
    if requester_id != target_user_id && !is_sys_admin {
        let school_id = get_admin_school_id(&state.db, &auth_user).await?;

        let target_user = sqlx::query!(
            "SELECT school_id FROM users WHERE id = $1",
            target_user_id.into_inner()
        )
        .fetch_one(&state.db)
        .await?;

        let target_school_id = target_user.school_id.map(SchoolId::from);
        if target_school_id != Some(school_id) {
            return Err(AppError::forbidden(
                "You can only view permissions for users in your school".to_string(),
            ));
        }
    }

    let permissions = service::get_user_permissions(&state.db, target_user_id).await?;

    Ok(Json(permissions))
}
