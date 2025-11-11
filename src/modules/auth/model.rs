use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::users::model::User;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    #[schema(example = "password123")]
    pub password: String,
}

// Login response
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub user: User,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequestDto {
    #[validate(length(min = 1))]
    #[schema(example = "John")]
    pub first_name: String,
    #[validate(length(min = 1))]
    #[schema(example = "Doe")]
    pub last_name: String,
    #[validate(email)]
    #[schema(example = "john@example.com")]
    pub email: String,
    #[validate(length(min = 8))]
    #[schema(example = "password123")]
    pub password: String,
}
