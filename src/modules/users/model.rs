use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

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

#[derive(Serialize, FromRow, Debug, ToSchema)]
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
