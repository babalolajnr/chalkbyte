use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Generate a slug from a name
/// Converts to lowercase, replaces spaces and hyphens with underscores,
/// removes invalid characters, and ensures it starts with a letter
pub fn generate_slug(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c == ' ' || c == '-' {
                '_'
            } else if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Remove consecutive underscores and trim underscores from ends
    let mut result = String::new();
    let mut prev_underscore = false;
    for c in slug.chars() {
        if c == '_' {
            if !prev_underscore && !result.is_empty() {
                result.push(c);
            }
            prev_underscore = true;
        } else {
            result.push(c);
            prev_underscore = false;
        }
    }

    // Trim trailing underscores
    result.trim_end_matches('_').to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub school_id: Option<Uuid>,
    pub is_system_role: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleWithPermissions {
    #[serde(flatten)]
    pub role: Role,
    pub permissions: Vec<Permission>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RolePermission {
    pub id: Uuid,
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserRole {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub assigned_by: Option<Uuid>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithRoles {
    pub user_id: Uuid,
    pub roles: Vec<RoleWithPermissions>,
}

// DTOs

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateRoleDto {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[validate(length(max = 500, message = "Description must not exceed 500 characters"))]
    pub description: Option<String>,
    /// If provided, creates a school-scoped role. If null and user is system admin, creates a system role.
    pub school_id: Option<Uuid>,
    /// Permission IDs to assign to this role
    pub permission_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateRoleDto {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,
    #[validate(length(max = 500, message = "Description must not exceed 500 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignPermissionsDto {
    pub permission_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRoleToUserDto {
    pub role_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RoleFilterParams {
    /// Filter by school_id (null for system roles)
    pub school_id: Option<Uuid>,
    /// Filter system roles only
    pub is_system_role: Option<bool>,
    /// Search by name
    #[allow(dead_code)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PermissionFilterParams {
    /// Filter by category
    pub category: Option<String>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedRolesResponse {
    pub data: Vec<RoleWithPermissions>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedPermissionsResponse {
    pub data: Vec<Permission>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleAssignmentResponse {
    pub message: String,
    pub user_id: Uuid,
    pub role_id: Uuid,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionCategory {
    pub category: String,
    pub permissions: Vec<Permission>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, ToSchema)]
pub struct GroupedPermissionsResponse {
    pub categories: Vec<PermissionCategory>,
}
