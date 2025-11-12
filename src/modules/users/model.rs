use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
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
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct CreateUserDto {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub role: Option<UserRole>,
}
