//! User data models and DTOs.
//!
//! This module contains all data structures related to user management,
//! including user entities, request/response DTOs, and system role definitions.
//!
//! # Core Types
//!
//! - [`User`] - Base user entity from the database
//! - [`UserWithRelations`] - User with joined school, level, branch, and roles
//! - [`UserWithSchool`] - User with school information only
//!
//! # Request DTOs
//!
//! - [`CreateUserDto`] - Create a new user
//! - [`UpdateProfileDto`] - Update user profile (name only)
//! - [`ChangePasswordDto`] - Change user password
//! - [`UserFilterParams`] - Query parameters for filtering users
//!
//! # System Roles
//!
//! The [`system_roles`] module provides constants and utilities for working
//! with the four system-defined roles:
//!
//! - System Admin (global access, CLI-created only)
//! - Admin (school-scoped management)
//! - Teacher (school-scoped, teaching permissions)
//! - Student (school-scoped, basic permissions)

use crate::utils::serde::deserialize_optional_uuid;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// A user in the system.
///
/// This struct represents the core user entity stored in the database.
/// Users are associated with a school (except system admins) and can
/// have multiple roles assigned.
#[derive(Serialize, Deserialize, FromRow, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub school_id: Option<Uuid>,
    pub level_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub grade_level: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// DTO for creating a new user.
///
/// Used by admins to create users within their scope. School admins
/// can only create users within their school, while system admins
/// can create users in any school.
#[derive(Deserialize, Debug, Clone, Validate, ToSchema)]
pub struct CreateUserDto {
    #[validate(length(min = 1))]
    pub first_name: String,
    #[validate(length(min = 1))]
    pub last_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    /// Role IDs to assign to the user. If empty, no roles are assigned.
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    pub school_id: Option<Uuid>,
}

/// A school entity.
///
/// Schools are the primary organizational unit. All non-system-admin
/// users are associated with exactly one school.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow, ToSchema)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// DTO for creating a new school.
///
/// Only system admins can create schools.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSchoolDto {
    pub name: String,
    pub address: Option<String>,
}

/// User with their associated school information.
///
/// Used in responses where both user and school data are needed.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithSchool {
    pub user: User,
    pub school: Option<School>,
}

/// Summary information about a role.
///
/// Used in user responses to include assigned role details.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub struct RoleInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system_role: bool,
}

/// Summary information about an educational level.
///
/// Used in user responses to include level details.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub struct LevelInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// Summary information about a school branch.
///
/// Used in user responses to include branch details.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub struct BranchInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// Summary information about a school.
///
/// Used in responses where full school details aren't needed.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub struct SchoolInfo {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
}

/// User with all related entities joined.
///
/// This is the most complete user representation, including school,
/// level, branch, and all assigned roles.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithRelations {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub grade_level: Option<String>,
    pub school: Option<SchoolInfo>,
    pub level: Option<LevelInfo>,
    pub branch: Option<BranchInfo>,
    pub roles: Vec<RoleInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Query parameters for filtering schools.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SchoolFilterParams {
    pub name: Option<String>,
    pub address: Option<String>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

/// Paginated response containing schools.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedSchoolsResponse {
    pub data: Vec<School>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

/// Query parameters for filtering users.
///
/// All filters are optional and can be combined.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UserFilterParams {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    /// Filter by role ID
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub role_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub school_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

/// Paginated response containing users with full relations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedUsersResponse {
    pub data: Vec<UserWithRelations>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

/// Paginated response containing basic user data.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedBasicUsersResponse {
    pub data: Vec<User>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

/// School information with user counts by role.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SchoolFullInfo {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub total_students: i64,
    pub total_teachers: i64,
    pub total_admins: i64,
}

/// DTO for updating user profile.
///
/// Only name fields can be updated through this DTO.
/// Email and other fields require different endpoints.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileDto {
    #[validate(length(min = 1))]
    pub first_name: Option<String>,
    #[validate(length(min = 1))]
    pub last_name: Option<String>,
}

/// DTO for changing user password.
///
/// Requires the current password for verification before
/// allowing the password change.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordDto {
    #[validate(length(min = 1))]
    #[serde(alias = "old_password")]
    pub current_password: String,
    #[validate(length(min = 8))]
    #[schema(example = "newPassword123")]
    pub new_password: String,
}

/// Well-known system role slugs and IDs.
///
/// This module provides constants and helper functions for working with
/// the four system-defined roles. These roles have fixed UUIDs and cannot
/// be deleted or modified.
///
/// # Role Hierarchy
///
/// ```text
/// System Admin (full system access)
///     └── Admin (school-scoped management)
///             └── Teacher (teaching permissions)
///                     └── Student (basic permissions)
/// ```
///
/// # Example
///
/// ```ignore
/// use crate::modules::users::model::system_roles;
///
/// // Check if a role is a system role
/// if system_roles::is_system_role(&role_id) {
///     // Handle system role
/// }
///
/// // Get role name for display
/// if let Some(name) = system_roles::get_name(&role_id) {
///     println!("Role: {}", name);
/// }
/// ```
pub mod system_roles {
    use uuid::Uuid;

    /// Role slugs - use these for lookups instead of hardcoded UUIDs
    #[allow(dead_code)]
    pub mod slugs {
        pub const SYSTEM_ADMIN: &str = "system_admin";
        pub const ADMIN: &str = "admin";
        pub const TEACHER: &str = "teacher";
        pub const STUDENT: &str = "student";
    }

    /// System Admin role - full system access
    pub const SYSTEM_ADMIN: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000001);
    /// Admin role - school-scoped management
    pub const ADMIN: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000002);
    /// Teacher role - teaching-related permissions
    pub const TEACHER: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000003);
    /// Student role - basic read permissions
    pub const STUDENT: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000004);

    /// Get all system role IDs
    pub fn all() -> Vec<Uuid> {
        vec![SYSTEM_ADMIN, ADMIN, TEACHER, STUDENT]
    }

    /// Get all system role slugs
    #[allow(dead_code)]
    pub fn all_slugs() -> Vec<&'static str> {
        vec![
            slugs::SYSTEM_ADMIN,
            slugs::ADMIN,
            slugs::TEACHER,
            slugs::STUDENT,
        ]
    }

    /// Check if a role ID is a system role
    pub fn is_system_role(role_id: &Uuid) -> bool {
        all().contains(role_id)
    }

    /// Check if a slug is a system role slug
    #[allow(dead_code)]
    pub fn is_system_role_slug(slug: &str) -> bool {
        all_slugs().contains(&slug)
    }

    /// Get role name by ID
    pub fn get_name(role_id: &Uuid) -> Option<&'static str> {
        match *role_id {
            id if id == SYSTEM_ADMIN => Some("System Admin"),
            id if id == ADMIN => Some("Admin"),
            id if id == TEACHER => Some("Teacher"),
            id if id == STUDENT => Some("Student"),
            _ => None,
        }
    }

    /// Get role slug by ID
    #[allow(dead_code)]
    pub fn get_slug(role_id: &Uuid) -> Option<&'static str> {
        match *role_id {
            id if id == SYSTEM_ADMIN => Some(slugs::SYSTEM_ADMIN),
            id if id == ADMIN => Some(slugs::ADMIN),
            id if id == TEACHER => Some(slugs::TEACHER),
            id if id == STUDENT => Some(slugs::STUDENT),
            _ => None,
        }
    }

    /// Get role ID by slug
    #[allow(dead_code)]
    pub fn get_id_by_slug(slug: &str) -> Option<Uuid> {
        match slug {
            slugs::SYSTEM_ADMIN => Some(SYSTEM_ADMIN),
            slugs::ADMIN => Some(ADMIN),
            slugs::TEACHER => Some(TEACHER),
            slugs::STUDENT => Some(STUDENT),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_system_roles_ids() {
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
    fn test_get_role_name() {
        assert_eq!(
            system_roles::get_name(&system_roles::SYSTEM_ADMIN),
            Some("System Admin")
        );
        assert_eq!(system_roles::get_name(&system_roles::ADMIN), Some("Admin"));
        assert_eq!(
            system_roles::get_name(&system_roles::TEACHER),
            Some("Teacher")
        );
        assert_eq!(
            system_roles::get_name(&system_roles::STUDENT),
            Some("Student")
        );
        assert_eq!(system_roles::get_name(&Uuid::new_v4()), None);
    }

    #[test]
    fn test_update_profile_dto_validation() {
        use validator::Validate;

        let dto = UpdateProfileDto {
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
        };
        assert!(dto.validate().is_ok());

        let dto_empty = UpdateProfileDto {
            first_name: Some("".to_string()),
            last_name: Some("Valid".to_string()),
        };
        assert!(dto_empty.validate().is_err());
    }

    #[test]
    fn test_change_password_dto_validation() {
        use validator::Validate;

        let dto = ChangePasswordDto {
            current_password: "currentPass".to_string(),
            new_password: "newPassword123".to_string(),
        };
        assert!(dto.validate().is_ok());

        let dto_short = ChangePasswordDto {
            current_password: "current".to_string(),
            new_password: "short".to_string(),
        };
        assert!(dto_short.validate().is_err());

        let dto_empty_current = ChangePasswordDto {
            current_password: "".to_string(),
            new_password: "validPassword123".to_string(),
        };
        assert!(dto_empty_current.validate().is_err());
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            school_id: None,
            level_id: None,
            branch_id: None,
            date_of_birth: None,
            grade_level: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&user).unwrap();
        assert!(serialized.contains("john@example.com"));
        assert!(serialized.contains("John"));
        assert!(serialized.contains("Doe"));
    }

    #[test]
    fn test_school_serialization() {
        let school = School {
            id: Uuid::new_v4(),
            name: "Test School".to_string(),
            address: Some("123 Main St".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&school).unwrap();
        assert!(serialized.contains("Test School"));
        assert!(serialized.contains("123 Main St"));
    }

    #[test]
    fn test_create_user_dto_deserialize() {
        let json = r#"{"first_name":"Jane","last_name":"Smith","email":"jane@test.com","password":"password123","role_ids":[],"school_id":null}"#;
        let dto: CreateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.first_name, "Jane");
        assert_eq!(dto.last_name, "Smith");
        assert_eq!(dto.email, "jane@test.com");
        assert_eq!(dto.password, "password123");
        assert!(dto.role_ids.is_empty());
    }

    #[test]
    fn test_create_user_dto_with_roles() {
        let role_id = Uuid::new_v4();
        let json = format!(
            r#"{{"first_name":"Jane","last_name":"Smith","email":"jane@test.com","password":"password123","role_ids":["{}"],"school_id":null}}"#,
            role_id
        );
        let dto: CreateUserDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto.role_ids.len(), 1);
        assert_eq!(dto.role_ids[0], role_id);
    }

    #[test]
    fn test_create_school_dto_deserialize() {
        let json = r#"{"name":"New School","address":"456 Oak Ave"}"#;
        let dto: CreateSchoolDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "New School");
        assert_eq!(dto.address, Some("456 Oak Ave".to_string()));
    }
}
