use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::users::model::User;

// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub role: String,
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

// MFA temporary token claims
#[derive(Debug, Serialize, Deserialize)]
pub struct MfaTempClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub mfa_pending: bool,
    pub exp: usize,
    pub iat: usize,
}

// Refresh token claims
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
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
    pub refresh_token: String,
    pub user: User,
}

// MFA required response (temp token for MFA verification)
#[derive(Debug, Serialize, ToSchema)]
pub struct MfaRequiredResponse {
    pub mfa_required: bool,
    pub temp_token: String,
}

// MFA verification request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MfaVerifyLoginRequest {
    #[validate(length(min = 1))]
    pub temp_token: String,
    #[validate(length(equal = 6))]
    #[schema(example = "123456")]
    pub code: String,
}

// MFA recovery code login request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MfaRecoveryLoginRequest {
    #[validate(length(min = 1))]
    pub temp_token: String,
    #[validate(length(equal = 8))]
    #[schema(example = "ABCD1234")]
    pub recovery_code: String,
}

// Refresh token request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
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
    #[serde(default)]
    pub role: Option<crate::modules::users::model::UserRole>,
}

// Forgot password request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    #[schema(example = "user@example.com")]
    pub email: String,
}

// Reset password request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8))]
    #[schema(example = "newPassword123")]
    pub new_password: String,
}

// Generic success message response
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
