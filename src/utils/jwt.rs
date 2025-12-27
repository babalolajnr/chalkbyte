use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::config::jwt::JwtConfig;
use crate::modules::auth::model::{Claims, MfaTempClaims, RefreshTokenClaims};
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

pub fn create_mfa_temp_token(
    user_id: Uuid,
    email: &str,
    jwt_config: &JwtConfig,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + 600; // 10 minutes expiry for MFA verification

    let claims = MfaTempClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        mfa_pending: true,
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_config.secret.as_bytes()),
    )
    .map_err(|e| AppError::internal_error(format!("Failed to create temp token: {}", e)))
}

pub fn verify_mfa_temp_token(
    token: &str,
    jwt_config: &JwtConfig,
) -> Result<MfaTempClaims, AppError> {
    let decoded = decode::<MfaTempClaims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::unauthorized("Invalid or expired temp token".to_string()))?;

    if !decoded.claims.mfa_pending {
        return Err(AppError::unauthorized("Invalid MFA temp token".to_string()));
    }

    Ok(decoded.claims)
}

pub fn create_refresh_token(
    user_id: Uuid,
    email: &str,
    jwt_config: &JwtConfig,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + jwt_config.refresh_token_expiry as usize;

    let claims = RefreshTokenClaims {
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
    .map_err(|e| AppError::internal_error(format!("Failed to create refresh token: {}", e)))
}

pub fn verify_refresh_token(
    token: &str,
    jwt_config: &JwtConfig,
) -> Result<RefreshTokenClaims, AppError> {
    decode::<RefreshTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::unauthorized("Invalid or expired refresh token".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_jwt_config() -> JwtConfig {
        JwtConfig {
            secret: "test_secret_key_for_testing_purposes".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        }
    }

    #[test]
    fn test_create_access_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let result = create_access_token(user_id, email, &jwt_config);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = create_access_token(user_id, email, &jwt_config).unwrap();
        let result = verify_token(&token, &jwt_config);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.email, email);
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn test_verify_token_invalid() {
        let jwt_config = get_test_jwt_config();
        let invalid_token = "invalid.token.here";

        let result = verify_token(invalid_token, &jwt_config);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = create_access_token(user_id, email, &jwt_config).unwrap();

        let wrong_jwt_config = JwtConfig {
            secret: "different_secret_key".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        };

        let result = verify_token(&token, &wrong_jwt_config);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_empty() {
        let jwt_config = get_test_jwt_config();
        let empty_token = "";

        let result = verify_token(empty_token, &jwt_config);

        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiry_is_set() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = create_access_token(user_id, email, &jwt_config).unwrap();
        let claims = verify_token(&token, &jwt_config).unwrap();

        assert!(claims.exp > claims.iat);
        assert_eq!(
            claims.exp - claims.iat,
            jwt_config.access_token_expiry as usize
        );
    }

    #[test]
    fn test_token_with_special_characters_in_email() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test+special@example.co.uk";

        let token = create_access_token(user_id, email, &jwt_config).unwrap();
        let claims = verify_token(&token, &jwt_config).unwrap();

        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_verify_token_malformed() {
        let jwt_config = get_test_jwt_config();
        let malformed_tokens = vec![
            "not.enough.parts",
            "too.many.parts.here.extra",
            "!!!.invalid.chars",
            "header.payload.",
            ".payload.signature",
        ];

        for token in malformed_tokens {
            let result = verify_token(token, &jwt_config);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_create_token_different_users_different_tokens() {
        let jwt_config = get_test_jwt_config();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        let email1 = "user1@example.com";
        let email2 = "user2@example.com";

        let token1 = create_access_token(user_id1, email1, &jwt_config).unwrap();
        let token2 = create_access_token(user_id2, email2, &jwt_config).unwrap();

        assert_ne!(token1, token2);

        let claims1 = verify_token(&token1, &jwt_config).unwrap();
        let claims2 = verify_token(&token2, &jwt_config).unwrap();

        assert_eq!(claims1.sub, user_id1.to_string());
        assert_eq!(claims2.sub, user_id2.to_string());
        assert_eq!(claims1.email, email1);
        assert_eq!(claims2.email, email2);
    }

    #[test]
    fn test_create_refresh_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let result = create_refresh_token(user_id, email, &jwt_config);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_refresh_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "refresh@example.com";

        let token = create_refresh_token(user_id, email, &jwt_config).unwrap();
        let result = verify_refresh_token(&token, &jwt_config);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.email, email);
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn test_create_mfa_temp_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "mfa@example.com";

        let result = create_mfa_temp_token(user_id, email, &jwt_config);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_mfa_temp_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "mfa@example.com";

        let token = create_mfa_temp_token(user_id, email, &jwt_config).unwrap();
        let result = verify_mfa_temp_token(&token, &jwt_config);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.email, email);
        assert_eq!(claims.sub, user_id.to_string());
        assert!(claims.mfa_pending);
    }

    #[test]
    fn test_verify_mfa_temp_token_invalid() {
        let jwt_config = get_test_jwt_config();
        let invalid_token = "invalid.mfa.token";

        let result = verify_mfa_temp_token(invalid_token, &jwt_config);

        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token_expiry_longer_than_access() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let access_token = create_access_token(user_id, email, &jwt_config).unwrap();
        let refresh_token = create_refresh_token(user_id, email, &jwt_config).unwrap();

        let access_claims = verify_token(&access_token, &jwt_config).unwrap();
        let refresh_claims = verify_refresh_token(&refresh_token, &jwt_config).unwrap();

        assert!(refresh_claims.exp > access_claims.exp);
    }
}
