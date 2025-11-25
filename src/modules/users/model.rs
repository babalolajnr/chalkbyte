use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    SystemAdmin,
    Admin,
    Teacher,
    Student,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::Student
    }
}

#[derive(Serialize, Deserialize, FromRow, Debug, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: UserRole,
    pub school_id: Option<Uuid>,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct CreateUserDto {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub role: Option<UserRole>,
    pub school_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSchoolDto {
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserWithSchool {
    pub user: User,
    pub school: Option<School>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SchoolFilterParams {
    pub name: Option<String>,
    pub address: Option<String>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedSchoolsResponse {
    pub data: Vec<School>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserFilterParams {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub school_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedUsersResponse {
    pub data: Vec<User>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SchoolFullInfo {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub total_students: i64,
    pub total_teachers: i64,
    pub total_admins: i64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileDto {
    #[validate(length(min = 1))]
    pub first_name: Option<String>,
    #[validate(length(min = 1))]
    pub last_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordDto {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(length(min = 8))]
    #[schema(example = "newPassword123")]
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_user_role_default() {
        let default_role = UserRole::default();
        assert_eq!(default_role, UserRole::Student);
    }

    #[test]
    fn test_user_role_equality() {
        assert_eq!(UserRole::Student, UserRole::Student);
        assert_eq!(UserRole::Teacher, UserRole::Teacher);
        assert_eq!(UserRole::Admin, UserRole::Admin);
        assert_eq!(UserRole::SystemAdmin, UserRole::SystemAdmin);
    }

    #[test]
    fn test_user_role_inequality() {
        assert_ne!(UserRole::Student, UserRole::Teacher);
        assert_ne!(UserRole::Teacher, UserRole::Admin);
        assert_ne!(UserRole::Admin, UserRole::SystemAdmin);
        assert_ne!(UserRole::Student, UserRole::SystemAdmin);
    }

    #[test]
    fn test_user_role_clone() {
        let role = UserRole::Admin;
        let cloned = role.clone();
        assert_eq!(role, cloned);
    }

    #[test]
    fn test_user_role_serialize_student() {
        let role = UserRole::Student;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""student""#);
    }

    #[test]
    fn test_user_role_serialize_teacher() {
        let role = UserRole::Teacher;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""teacher""#);
    }

    #[test]
    fn test_user_role_serialize_admin() {
        let role = UserRole::Admin;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""admin""#);
    }

    #[test]
    fn test_user_role_serialize_system_admin() {
        let role = UserRole::SystemAdmin;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, r#""system_admin""#);
    }

    #[test]
    fn test_user_role_deserialize_student() {
        let json = r#""student""#;
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::Student);
    }

    #[test]
    fn test_user_role_deserialize_teacher() {
        let json = r#""teacher""#;
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::Teacher);
    }

    #[test]
    fn test_user_role_deserialize_admin() {
        let json = r#""admin""#;
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::Admin);
    }

    #[test]
    fn test_user_role_deserialize_system_admin() {
        let json = r#""system_admin""#;
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::SystemAdmin);
    }

    #[test]
    fn test_user_role_deserialize_invalid() {
        let json = r#""invalid_role""#;
        let result: Result<UserRole, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_role_serialize_deserialize_round_trip() {
        let roles = vec![
            UserRole::Student,
            UserRole::Teacher,
            UserRole::Admin,
            UserRole::SystemAdmin,
        ];

        for original_role in roles {
            let serialized = serde_json::to_string(&original_role).unwrap();
            let deserialized: UserRole = serde_json::from_str(&serialized).unwrap();
            assert_eq!(original_role, deserialized);
        }
    }

    #[test]
    fn test_user_role_pattern_matching() {
        let role = UserRole::Admin;

        let result = match role {
            UserRole::SystemAdmin => "system_admin",
            UserRole::Admin => "admin",
            UserRole::Teacher => "teacher",
            UserRole::Student => "student",
        };

        assert_eq!(result, "admin");
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
            role: UserRole::Student,
            school_id: None,
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
        };

        let serialized = serde_json::to_string(&school).unwrap();
        assert!(serialized.contains("Test School"));
        assert!(serialized.contains("123 Main St"));
    }

    #[test]
    fn test_create_user_dto_deserialize() {
        let json = r#"{"first_name":"Jane","last_name":"Smith","email":"jane@test.com","role":"teacher","school_id":null}"#;
        let dto: CreateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.first_name, "Jane");
        assert_eq!(dto.last_name, "Smith");
        assert_eq!(dto.email, "jane@test.com");
        assert_eq!(dto.role, Some(UserRole::Teacher));
    }

    #[test]
    fn test_create_school_dto_deserialize() {
        let json = r#"{"name":"New School","address":"456 Oak Ave"}"#;
        let dto: CreateSchoolDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "New School");
        assert_eq!(dto.address, Some("456 Oak Ave".to_string()));
    }
}
