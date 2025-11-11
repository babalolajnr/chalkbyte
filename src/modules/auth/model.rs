use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::modules::users::model::User;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

// Password reset token structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ResetTokenClaims {
    pub user_id: String,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

// Login request structure
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub user: User,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequestDto {
    #[validate(length(min = 1))]
    pub first_name: String,
    #[validate(length(min = 1))]
    pub last_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}
