use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, FromRow, Debug, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct CreateUserDto {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}
