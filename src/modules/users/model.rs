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
