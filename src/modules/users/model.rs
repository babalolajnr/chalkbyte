use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateUserDto {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}
