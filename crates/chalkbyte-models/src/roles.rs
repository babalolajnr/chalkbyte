//! Role and permission domain models and DTOs.
//!
//! This module contains all data structures related to role-based access control,
//! including roles, permissions, and their relationships.

use crate::ids::{PermissionId, RoleId, RolePermissionId, SchoolId, UserId, UserRoleId};
use chalkbyte_core::PaginationParams;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
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
    pub id: PermissionId,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub school_id: Option<SchoolId>,
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
    pub id: RolePermissionId,
    pub role_id: RoleId,
    pub permission_id: PermissionId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserRole {
    pub id: UserRoleId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub assigned_by: Option<UserId>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithRoles {
    pub user_id: UserId,
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
    pub school_id: Option<SchoolId>,
    /// Permission IDs to assign to this role
    pub permission_ids: Option<Vec<PermissionId>>,
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
    pub permission_ids: Vec<PermissionId>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRoleToUserDto {
    pub role_id: RoleId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RoleFilterParams {
    /// Filter by school_id (null for system roles)
    pub school_id: Option<SchoolId>,
    /// Filter system roles only
    pub is_system_role: Option<bool>,
    /// Search by name
    #[allow(dead_code)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PermissionFilterParams {
    /// Filter by category
    pub category: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedRolesResponse {
    pub data: Vec<RoleWithPermissions>,
    pub meta: chalkbyte_core::PaginationMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedPermissionsResponse {
    pub data: Vec<Permission>,
    pub meta: chalkbyte_core::PaginationMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleAssignmentResponse {
    pub message: String,
    pub user_id: UserId,
    pub role_id: RoleId,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("System Admin"), "system_admin");
        assert_eq!(generate_slug("My-Role"), "my_role");
        assert_eq!(generate_slug("Role  Name"), "role_name");
        assert_eq!(generate_slug("Role123"), "role123");
    }

    #[test]
    fn test_create_role_dto_validation() {
        let valid_dto = CreateRoleDto {
            name: "Test Role".to_string(),
            description: Some("A test role".to_string()),
            school_id: None,
            permission_ids: None,
        };
        assert!(valid_dto.validate().is_ok());

        let empty_name = CreateRoleDto {
            name: "".to_string(),
            description: None,
            school_id: None,
            permission_ids: None,
        };
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_update_role_dto_validation() {
        let valid_dto = UpdateRoleDto {
            name: Some("Updated Role".to_string()),
            description: None,
        };
        assert!(valid_dto.validate().is_ok());

        let long_description = UpdateRoleDto {
            name: None,
            description: Some("x".repeat(501)),
        };
        assert!(long_description.validate().is_err());
    }
}
