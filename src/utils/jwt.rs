use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::config::jwt::JwtConfig;
use crate::modules::auth::model::Claims;
use crate::utils::errors::AppError;

pub fn create_access_token(
    user_id: Uuid,
    email: &str,
    role: &crate::modules::users::model::UserRole,
    jwt_config: &JwtConfig,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + jwt_config.access_token_expiry as usize;

    let role_str = match role {
        crate::modules::users::model::UserRole::Admin => "admin",
        crate::modules::users::model::UserRole::Teacher => "teacher",
        crate::modules::users::model::UserRole::Student => "student",
    };

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role_str.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_config.secret.as_bytes()),
    )
    .map_err(|e| AppError::internal_error(format!("Failed to create token: {}", e)))
}

pub fn verify_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::unauthorized("Invalid or expired token".to_string()))
}
