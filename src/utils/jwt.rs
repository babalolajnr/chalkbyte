use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::config::jwt::JwtConfig;
use crate::modules::auth::model::Claims;
use crate::utils::errors::AppError;

pub fn create_access_token(
    user_id: Uuid,
    email: &str,
    jwt_config: &JwtConfig,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + jwt_config.access_token_expiry as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_config.secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("Failed to create token: {}", e)))
}

pub fn verify_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))
}
