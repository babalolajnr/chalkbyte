use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

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
pub struct CustomRole {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub school_id: Option<Uuid>,
    pub is_system_role: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomRoleWithPermissions {
    #[serde(flatten)]
    pub role: CustomRole,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RolePermission {
    pub id: Uuid,
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserCustomRole {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub assigned_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithCustomRoles {
    pub user_id: Uuid,
    pub roles: Vec<CustomRoleWithPermissions>,
}

// DTOs

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCustomRoleDto {
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
pub struct UpdateCustomRoleDto {
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
    pub data: Vec<CustomRoleWithPermissions>,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionCategory {
    pub category: String,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GroupedPermissionsResponse {
    pub categories: Vec<PermissionCategory>,
}
