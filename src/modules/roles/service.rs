use anyhow::anyhow;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::utils::errors::AppError;
use crate::utils::pagination::PaginationMeta;

use super::model::{
    CreateRoleDto, PaginatedPermissionsResponse, PaginatedRolesResponse, Permission,
    PermissionFilterParams, Role, RoleAssignmentResponse, RoleFilterParams, RoleWithPermissions,
    UpdateRoleDto,
};

// ============ Permission Services ============

#[instrument(skip(db))]
pub async fn get_all_permissions(
    db: &PgPool,
    params: PermissionFilterParams,
) -> Result<PaginatedPermissionsResponse, AppError> {
    let limit = params.pagination.limit();
    let offset = params.pagination.offset();

    let mut query = String::from(
        "SELECT id, name, description, category, created_at, updated_at FROM permissions WHERE 1=1",
    );
    let mut count_query = String::from("SELECT COUNT(*) FROM permissions WHERE 1=1");

    if let Some(ref category) = params.category {
        query.push_str(&format!(" AND category = '{}'", category));
        count_query.push_str(&format!(" AND category = '{}'", category));
    }

    query.push_str(&format!(
        " ORDER BY category, name LIMIT {} OFFSET {}",
        limit, offset
    ));

    let permissions: Vec<Permission> = sqlx::query_as(&query).fetch_all(db).await?;

    let total: (i64,) = sqlx::query_as(&count_query).fetch_one(db).await?;
    let has_more = offset + (permissions.len() as i64) < total.0;

    let meta = PaginationMeta {
        total: total.0,
        limit,
        offset: Some(offset),
        page: params.pagination.page(),
        has_more,
    };

    Ok(PaginatedPermissionsResponse {
        data: permissions,
        meta,
    })
}

#[instrument(skip(db))]
pub async fn get_permission_by_id(db: &PgPool, id: Uuid) -> Result<Permission, AppError> {
    sqlx::query_as!(
        Permission,
        r#"SELECT id, name, description, category, created_at, updated_at
        FROM permissions WHERE id = $1"#,
        id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found(anyhow!("Permission not found")))
}

#[instrument(skip(db))]
pub async fn get_permissions_by_ids(
    db: &PgPool,
    ids: &[Uuid],
) -> Result<Vec<Permission>, AppError> {
    let permissions = sqlx::query_as!(
        Permission,
        r#"SELECT id, name, description, category, created_at, updated_at
        FROM permissions WHERE id = ANY($1)"#,
        ids
    )
    .fetch_all(db)
    .await?;

    Ok(permissions)
}

// ============ Role Services ============

#[instrument(skip(db))]
pub async fn create_role(
    db: &PgPool,
    dto: CreateRoleDto,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
    _created_by: Uuid,
) -> Result<RoleWithPermissions, AppError> {
    // Determine if this is a system role
    let is_system_role = is_system_admin && dto.school_id.is_none();

    // School admins can only create roles for their school
    let school_id = if is_system_admin {
        dto.school_id
    } else {
        requester_school_id
    };

    // Validate school_id for non-system admins
    if !is_system_admin && school_id.is_none() {
        return Err(AppError::bad_request(anyhow!(
            "School admin must have a school_id"
        )));
    }

    // Create the role
    let role = sqlx::query_as!(
        Role,
        r#"INSERT INTO roles (name, description, school_id, is_system_role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, school_id, is_system_role, created_at, updated_at"#,
        dto.name,
        dto.description,
        school_id,
        is_system_role
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return AppError::bad_request(anyhow!(
                    "A role with this name already exists in this scope"
                ));
            }
        }
        AppError::from(e)
    })?;

    // Assign permissions if provided
    let permissions = if let Some(permission_ids) = dto.permission_ids {
        assign_permissions_to_role_internal(db, role.id, &permission_ids).await?
    } else {
        vec![]
    };

    Ok(RoleWithPermissions { role, permissions })
}

#[instrument(skip(db))]
pub async fn get_roles(
    db: &PgPool,
    params: RoleFilterParams,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<PaginatedRolesResponse, AppError> {
    let limit = params.pagination.limit();
    let offset = params.pagination.offset();

    // Build dynamic query based on filters and requester permissions
    let roles: Vec<Role> = if is_system_admin {
        // System admin can see all roles
        if let Some(is_system) = params.is_system_role {
            if is_system {
                sqlx::query_as!(
                    Role,
                    r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
                    FROM roles WHERE is_system_role = true
                    ORDER BY name LIMIT $1 OFFSET $2"#,
                    limit,
                    offset
                )
                .fetch_all(db)
                .await?
            } else if let Some(school_id) = params.school_id {
                sqlx::query_as!(
                    Role,
                    r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
                    FROM roles WHERE school_id = $1
                    ORDER BY name LIMIT $2 OFFSET $3"#,
                    school_id,
                    limit,
                    offset
                )
                .fetch_all(db)
                .await?
            } else {
                sqlx::query_as!(
                    Role,
                    r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
                    FROM roles WHERE is_system_role = false
                    ORDER BY name LIMIT $1 OFFSET $2"#,
                    limit,
                    offset
                )
                .fetch_all(db)
                .await?
            }
        } else if let Some(school_id) = params.school_id {
            sqlx::query_as!(
                Role,
                r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
                FROM roles WHERE school_id = $1
                ORDER BY name LIMIT $2 OFFSET $3"#,
                school_id,
                limit,
                offset
            )
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as!(
                Role,
                r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
                FROM roles ORDER BY name LIMIT $1 OFFSET $2"#,
                limit,
                offset
            )
            .fetch_all(db)
            .await?
        }
    } else {
        // School admin can only see roles for their school
        let school_id = requester_school_id
            .ok_or_else(|| AppError::bad_request(anyhow!("School admin must have a school_id")))?;

        sqlx::query_as!(
            Role,
            r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
            FROM roles WHERE school_id = $1
            ORDER BY name LIMIT $2 OFFSET $3"#,
            school_id,
            limit,
            offset
        )
        .fetch_all(db)
        .await?
    };

    // Get total count
    let total: i64 = if is_system_admin {
        if let Some(is_system) = params.is_system_role {
            if is_system {
                sqlx::query_scalar!("SELECT COUNT(*) FROM roles WHERE is_system_role = true")
                    .fetch_one(db)
                    .await?
                    .unwrap_or(0)
            } else if let Some(school_id) = params.school_id {
                sqlx::query_scalar!("SELECT COUNT(*) FROM roles WHERE school_id = $1", school_id)
                    .fetch_one(db)
                    .await?
                    .unwrap_or(0)
            } else {
                sqlx::query_scalar!("SELECT COUNT(*) FROM roles WHERE is_system_role = false")
                    .fetch_one(db)
                    .await?
                    .unwrap_or(0)
            }
        } else if let Some(school_id) = params.school_id {
            sqlx::query_scalar!("SELECT COUNT(*) FROM roles WHERE school_id = $1", school_id)
                .fetch_one(db)
                .await?
                .unwrap_or(0)
        } else {
            sqlx::query_scalar!("SELECT COUNT(*) FROM roles")
                .fetch_one(db)
                .await?
                .unwrap_or(0)
        }
    } else {
        let school_id = requester_school_id.unwrap();
        sqlx::query_scalar!("SELECT COUNT(*) FROM roles WHERE school_id = $1", school_id)
            .fetch_one(db)
            .await?
            .unwrap_or(0)
    };

    // Fetch permissions for each role
    let mut roles_with_permissions = Vec::new();
    for role in roles.iter() {
        let permissions = get_role_permissions(db, role.id).await?;
        roles_with_permissions.push(RoleWithPermissions {
            role: role.clone(),
            permissions,
        });
    }

    let has_more = offset + (roles.len() as i64) < total;

    let meta = PaginationMeta {
        total,
        limit,
        offset: Some(offset),
        page: params.pagination.page(),
        has_more,
    };

    Ok(PaginatedRolesResponse {
        data: roles_with_permissions,
        meta,
    })
}

#[instrument(skip(db))]
pub async fn get_role_by_id(
    db: &PgPool,
    id: Uuid,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<RoleWithPermissions, AppError> {
    let role = sqlx::query_as!(
        Role,
        r#"SELECT id, name, description, school_id, is_system_role, created_at, updated_at
        FROM roles WHERE id = $1"#,
        id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found(anyhow!("Role not found")))?;

    // Authorization check
    if !is_system_admin {
        if role.is_system_role {
            return Err(AppError::forbidden(
                "School admins cannot access system roles".to_string(),
            ));
        }
        if role.school_id != requester_school_id {
            return Err(AppError::forbidden(
                "You can only access roles from your school".to_string(),
            ));
        }
    }

    let permissions = get_role_permissions(db, role.id).await?;

    Ok(RoleWithPermissions { role, permissions })
}

#[instrument(skip(db))]
pub async fn update_role(
    db: &PgPool,
    id: Uuid,
    dto: UpdateRoleDto,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<RoleWithPermissions, AppError> {
    // First verify the role exists and user has access
    let existing = get_role_by_id(db, id, requester_school_id, is_system_admin).await?;

    let name = dto.name.unwrap_or(existing.role.name);
    let description = dto.description.or(existing.role.description);

    let role = sqlx::query_as!(
        Role,
        r#"UPDATE roles SET name = $1, description = $2, updated_at = NOW()
        WHERE id = $3
        RETURNING id, name, description, school_id, is_system_role, created_at, updated_at"#,
        name,
        description,
        id
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return AppError::bad_request(anyhow!(
                    "A role with this name already exists in this scope"
                ));
            }
        }
        AppError::from(e)
    })?;

    let permissions = get_role_permissions(db, role.id).await?;

    Ok(RoleWithPermissions { role, permissions })
}

#[instrument(skip(db))]
pub async fn delete_role(
    db: &PgPool,
    id: Uuid,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<(), AppError> {
    // First verify the role exists and user has access
    let _ = get_role_by_id(db, id, requester_school_id, is_system_admin).await?;

    sqlx::query!("DELETE FROM roles WHERE id = $1", id)
        .execute(db)
        .await?;

    Ok(())
}

// ============ Role Permissions Services ============

#[instrument(skip(db))]
pub async fn get_role_permissions(db: &PgPool, role_id: Uuid) -> Result<Vec<Permission>, AppError> {
    let permissions = sqlx::query_as!(
        Permission,
        r#"SELECT p.id, p.name, p.description, p.category, p.created_at, p.updated_at
        FROM permissions p
        INNER JOIN role_permissions rp ON p.id = rp.permission_id
        WHERE rp.role_id = $1
        ORDER BY p.category, p.name"#,
        role_id
    )
    .fetch_all(db)
    .await?;

    Ok(permissions)
}

async fn assign_permissions_to_role_internal(
    db: &PgPool,
    role_id: Uuid,
    permission_ids: &[Uuid],
) -> Result<Vec<Permission>, AppError> {
    // Insert permissions (ignore duplicates)
    for permission_id in permission_ids {
        sqlx::query!(
            r#"INSERT INTO role_permissions (role_id, permission_id)
            VALUES ($1, $2)
            ON CONFLICT (role_id, permission_id) DO NOTHING"#,
            role_id,
            permission_id
        )
        .execute(db)
        .await?;
    }

    get_role_permissions(db, role_id).await
}

#[instrument(skip(db))]
pub async fn assign_permissions_to_role(
    db: &PgPool,
    role_id: Uuid,
    permission_ids: Vec<Uuid>,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<RoleWithPermissions, AppError> {
    // Verify role exists and user has access
    let role = get_role_by_id(db, role_id, requester_school_id, is_system_admin).await?;

    // Verify all permission IDs exist
    let existing_permissions = get_permissions_by_ids(db, &permission_ids).await?;
    if existing_permissions.len() != permission_ids.len() {
        return Err(AppError::bad_request(anyhow!(
            "One or more permission IDs are invalid"
        )));
    }

    let permissions = assign_permissions_to_role_internal(db, role_id, &permission_ids).await?;

    Ok(RoleWithPermissions {
        role: role.role,
        permissions,
    })
}

#[instrument(skip(db))]
pub async fn remove_permission_from_role(
    db: &PgPool,
    role_id: Uuid,
    permission_id: Uuid,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<RoleWithPermissions, AppError> {
    // Verify role exists and user has access
    let role = get_role_by_id(db, role_id, requester_school_id, is_system_admin).await?;

    sqlx::query!(
        "DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2",
        role_id,
        permission_id
    )
    .execute(db)
    .await?;

    let permissions = get_role_permissions(db, role_id).await?;

    Ok(RoleWithPermissions {
        role: role.role,
        permissions,
    })
}

// ============ User Role Assignment Services ============

#[instrument(skip(db))]
pub async fn assign_role_to_user(
    db: &PgPool,
    user_id: Uuid,
    role_id: Uuid,
    assigned_by: Uuid,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<RoleAssignmentResponse, AppError> {
    // Verify role exists and requester has access
    let role = get_role_by_id(db, role_id, requester_school_id, is_system_admin).await?;

    // Verify the target user exists
    let target_user = sqlx::query!("SELECT id, school_id FROM users WHERE id = $1", user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow!("User not found")))?;

    // Authorization checks for non-system admins
    if !is_system_admin {
        if role.role.is_system_role {
            return Err(AppError::forbidden(
                "School admins cannot assign system roles".to_string(),
            ));
        }
    }

    // For system roles, allow assignment to users without school_id or with school_id
    // For school roles, verify the user is in the same school as the role
    if !role.role.is_system_role && role.role.school_id != target_user.school_id {
        return Err(AppError::bad_request(anyhow!(
            "School roles can only be assigned to users in that school"
        )));
    }

    sqlx::query!(
        r#"INSERT INTO user_roles (user_id, role_id, assigned_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, role_id) DO NOTHING"#,
        user_id,
        role_id,
        assigned_by
    )
    .execute(db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return AppError::bad_request(anyhow!("User already has this role"));
            }
        }
        AppError::from(e)
    })?;

    Ok(RoleAssignmentResponse {
        message: "Role assigned successfully".to_string(),
        user_id,
        role_id,
    })
}

#[instrument(skip(db))]
pub async fn remove_role_from_user(
    db: &PgPool,
    user_id: Uuid,
    role_id: Uuid,
    requester_school_id: Option<Uuid>,
    is_system_admin: bool,
) -> Result<(), AppError> {
    // Verify role exists and requester has access
    let role = get_role_by_id(db, role_id, requester_school_id, is_system_admin).await?;

    // Verify the target user exists and is accessible
    let target_user = sqlx::query!("SELECT id, school_id FROM users WHERE id = $1", user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow!("User not found")))?;

    // Authorization checks
    if !is_system_admin {
        if target_user.school_id != requester_school_id {
            return Err(AppError::forbidden(
                "You can only manage roles for users in your school".to_string(),
            ));
        }
        if role.role.is_system_role {
            return Err(AppError::forbidden(
                "School admins cannot manage system roles".to_string(),
            ));
        }
    }

    let result = sqlx::query!(
        "DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2",
        user_id,
        role_id
    )
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(anyhow!(
            "User does not have this role assigned"
        )));
    }

    Ok(())
}

/// Internal function for fetching user roles without authorization checks
/// Used by auth service during login
#[instrument(skip(db))]
pub async fn get_user_roles_internal(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<RoleWithPermissions>, AppError> {
    let roles = sqlx::query_as!(
        Role,
        r#"SELECT r.id, r.name, r.description, r.school_id, r.is_system_role, r.created_at, r.updated_at
        FROM roles r
        INNER JOIN user_roles ur ON r.id = ur.role_id
        WHERE ur.user_id = $1
        ORDER BY r.name"#,
        user_id
    )
    .fetch_all(db)
    .await?;

    let mut roles_with_permissions = Vec::new();
    for role in roles {
        let permissions = get_role_permissions(db, role.id).await?;
        roles_with_permissions.push(RoleWithPermissions { role, permissions });
    }

    Ok(roles_with_permissions)
}

#[instrument(skip(db))]
pub async fn get_user_roles(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<RoleWithPermissions>, AppError> {
    let roles = sqlx::query_as!(
        Role,
        r#"SELECT r.id, r.name, r.description, r.school_id, r.is_system_role, r.created_at, r.updated_at
        FROM roles r
        INNER JOIN user_roles ur ON r.id = ur.role_id
        WHERE ur.user_id = $1
        ORDER BY r.name"#,
        user_id
    )
    .fetch_all(db)
    .await?;

    let mut roles_with_permissions = Vec::new();
    for role in roles {
        let permissions = get_role_permissions(db, role.id).await?;
        roles_with_permissions.push(RoleWithPermissions { role, permissions });
    }

    Ok(roles_with_permissions)
}

// ============ Permission Check Services ============

#[instrument(skip(db))]
pub async fn user_has_permission(
    db: &PgPool,
    user_id: Uuid,
    permission_name: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            INNER JOIN role_permissions rp ON ur.role_id = rp.role_id
            INNER JOIN permissions p ON rp.permission_id = p.id
            WHERE ur.user_id = $1 AND p.name = $2
        )"#,
        user_id,
        permission_name
    )
    .fetch_one(db)
    .await?;

    Ok(result.unwrap_or(false))
}

#[instrument(skip(db))]
pub async fn get_user_permissions(db: &PgPool, user_id: Uuid) -> Result<Vec<Permission>, AppError> {
    let permissions = sqlx::query_as!(
        Permission,
        r#"SELECT DISTINCT p.id, p.name, p.description, p.category, p.created_at, p.updated_at
        FROM permissions p
        INNER JOIN role_permissions rp ON p.id = rp.permission_id
        INNER JOIN user_roles ur ON rp.role_id = ur.role_id
        WHERE ur.user_id = $1
        ORDER BY p.category, p.name"#,
        user_id
    )
    .fetch_all(db)
    .await?;

    Ok(permissions)
}
