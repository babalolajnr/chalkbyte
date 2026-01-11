//! JWT (JSON Web Token) utilities for authentication.
//!
//! This module provides functions for creating and verifying JWT tokens
//! used for authentication in the Chalkbyte API. It supports:
//!
//! - **Access tokens**: Short-lived tokens for API authentication
//! - **Refresh tokens**: Long-lived tokens for obtaining new access tokens
//! - **MFA temporary tokens**: Short-lived tokens for multi-factor authentication flow
//!
//! # Token Structure
//!
//! Access tokens include:
//! - User ID and email
//! - School ID (for school-scoped users)
//! - Role IDs assigned to the user
//! - Permission names derived from roles
//!
//! # Example
//!
//! ```ignore
//! use crate::utils::jwt::{create_access_token, verify_token};
//! use crate::config::jwt::JwtConfig;
//!
//! let config = JwtConfig::from_env();
//!
//! // Create a token
//! let token = create_access_token(
//!     user_id,
//!     "user@example.com",
//!     Some(school_id),
//!     vec![role_id],
//!     vec!["users:read".to_string()],
//!     &config,
//! )?;
//!
//! // Verify the token
//! let claims = verify_token(&token, &config)?;
//! ```

use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::config::jwt::JwtConfig;
use crate::modules::auth::model::{Claims, MfaTempClaims, RefreshTokenClaims};
use crate::utils::errors::AppError;

/// Creates an access token with embedded roles and permissions for permission-based access control.
///
/// The access token is a short-lived JWT that contains all the information needed
/// for authentication and authorization decisions without additional database queries.
///
/// # Arguments
///
/// * `user_id` - The user's UUID
/// * `email` - The user's email address
/// * `school_id` - Optional school ID for school-scoped users (None for system admins)
/// * `role_ids` - List of role IDs assigned to the user
/// * `permissions` - List of permission names (e.g., "users:create", "schools:read")
/// * `jwt_config` - JWT configuration containing the secret and expiry settings
///
/// # Returns
///
/// Returns the encoded JWT string on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns an error if token encoding fails (e.g., invalid secret key).
///
/// # Example
///
/// ```ignore
/// let token = create_access_token(
///     user_id,
///     "admin@school.com",
///     Some(school_id),
///     vec![admin_role_id],
///     vec!["users:create".to_string(), "users:read".to_string()],
///     &jwt_config,
/// )?;
/// ```
pub fn create_access_token(
    user_id: Uuid,
    email: &str,
    school_id: Option<Uuid>,
    role_ids: Vec<Uuid>,
    permissions: Vec<String>,
    jwt_config: &JwtConfig,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + jwt_config.access_token_expiry as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        school_id,
        role_ids,
        permissions,
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

/// Verifies an access token and returns the embedded claims.
///
/// This function validates the token signature and expiration, then extracts
/// the claims for use in authentication and authorization.
///
/// # Arguments
///
/// * `token` - The JWT string to verify
/// * `jwt_config` - JWT configuration containing the secret
///
/// # Returns
///
/// Returns the decoded [`Claims`] on success, or an [`AppError`] on failure.
///
/// # Errors
///
/// Returns an unauthorized error if:
/// - The token signature is invalid
/// - The token has expired
/// - The token is malformed
///
/// # Example
///
/// ```ignore
/// let claims = verify_token(&token, &jwt_config)?;
/// println!("User ID: {}", claims.sub);
/// println!("Permissions: {:?}", claims.permissions);
/// ```
pub fn verify_token(token: &str, jwt_config: &JwtConfig) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::unauthorized("Invalid or expired token".to_string()))
}

/// Creates a temporary token for MFA verification flow.
///
/// This token is issued after successful password authentication when MFA is enabled.
/// It has a short expiry (10 minutes) and can only be used to complete the MFA flow.
///
/// # Arguments
///
/// * `user_id` - The user's UUID
/// * `email` - The user's email address
/// * `jwt_config` - JWT configuration containing the secret
///
/// # Returns
///
/// Returns the encoded temporary JWT string on success.
///
/// # Errors
///
/// Returns an error if token encoding fails.
///
/// # Security Note
///
/// This token has `mfa_pending: true` flag and should only be accepted by
/// MFA verification endpoints, not regular API endpoints.
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

/// Verifies an MFA temporary token and returns the claims.
///
/// This function validates that the token is a valid MFA temporary token
/// with the `mfa_pending` flag set to true.
///
/// # Arguments
///
/// * `token` - The temporary JWT string to verify
/// * `jwt_config` - JWT configuration containing the secret
///
/// # Returns
///
/// Returns the decoded [`MfaTempClaims`] on success.
///
/// # Errors
///
/// Returns an unauthorized error if:
/// - The token is invalid or expired
/// - The `mfa_pending` flag is not true
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

/// Creates a refresh token for obtaining new access tokens.
///
/// Refresh tokens are long-lived and should be stored securely by the client.
/// They can be used to obtain new access tokens without re-authenticating.
///
/// # Arguments
///
/// * `user_id` - The user's UUID
/// * `email` - The user's email address
/// * `jwt_config` - JWT configuration containing the secret and refresh token expiry
///
/// # Returns
///
/// Returns the encoded refresh JWT string on success.
///
/// # Errors
///
/// Returns an error if token encoding fails.
///
/// # Security Note
///
/// Refresh tokens should be:
/// - Stored securely (e.g., HttpOnly cookies or secure storage)
/// - Rotated on use (issue new refresh token with each access token refresh)
/// - Revocable server-side for logout functionality
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

/// Verifies a refresh token and returns the claims.
///
/// # Arguments
///
/// * `token` - The refresh JWT string to verify
/// * `jwt_config` - JWT configuration containing the secret
///
/// # Returns
///
/// Returns the decoded [`RefreshTokenClaims`] on success.
///
/// # Errors
///
/// Returns an unauthorized error if the token is invalid or expired.
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
        let school_id = Some(Uuid::new_v4());
        let role_ids = vec![Uuid::new_v4()];
        let permissions = vec!["users:read".to_string(), "users:create".to_string()];

        let result = create_access_token(
            user_id,
            email,
            school_id,
            role_ids.clone(),
            permissions.clone(),
            &jwt_config,
        );

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_token_success() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let school_id = Some(Uuid::new_v4());
        let role_ids = vec![Uuid::new_v4()];
        let permissions = vec!["users:read".to_string()];

        let token = create_access_token(
            user_id,
            email,
            school_id,
            role_ids.clone(),
            permissions.clone(),
            &jwt_config,
        )
        .unwrap();
        let result = verify_token(&token, &jwt_config);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.email, email);
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.school_id, school_id);
        assert_eq!(claims.role_ids, role_ids);
        assert_eq!(claims.permissions, permissions);
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

        let token = create_access_token(user_id, email, None, vec![], vec![], &jwt_config).unwrap();

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

        let token = create_access_token(user_id, email, None, vec![], vec![], &jwt_config).unwrap();
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

        let token = create_access_token(user_id, email, None, vec![], vec![], &jwt_config).unwrap();
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

        let token1 =
            create_access_token(user_id1, email1, None, vec![], vec![], &jwt_config).unwrap();
        let token2 =
            create_access_token(user_id2, email2, None, vec![], vec![], &jwt_config).unwrap();

        assert_ne!(token1, token2);

        let claims1 = verify_token(&token1, &jwt_config).unwrap();
        let claims2 = verify_token(&token2, &jwt_config).unwrap();

        assert_eq!(claims1.sub, user_id1.to_string());
        assert_eq!(claims2.sub, user_id2.to_string());
        assert_eq!(claims1.email, email1);
        assert_eq!(claims2.email, email2);
    }

    #[test]
    fn test_token_with_permissions() {
        let jwt_config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let email = "admin@example.com";
        let school_id = Some(Uuid::new_v4());
        let role_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let permissions = vec![
            "users:create".to_string(),
            "users:read".to_string(),
            "users:update".to_string(),
            "schools:read".to_string(),
        ];

        let token = create_access_token(
            user_id,
            email,
            school_id,
            role_ids.clone(),
            permissions.clone(),
            &jwt_config,
        )
        .unwrap();

        let claims = verify_token(&token, &jwt_config).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.school_id, school_id);
        assert_eq!(claims.role_ids.len(), 2);
        assert_eq!(claims.permissions.len(), 4);
        assert!(claims.permissions.contains(&"users:create".to_string()));
        assert!(claims.permissions.contains(&"schools:read".to_string()));
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

        let access_token =
            create_access_token(user_id, email, None, vec![], vec![], &jwt_config).unwrap();
        let refresh_token = create_refresh_token(user_id, email, &jwt_config).unwrap();

        let access_claims = verify_token(&access_token, &jwt_config).unwrap();
        let refresh_claims = verify_refresh_token(&refresh_token, &jwt_config).unwrap();

        assert!(refresh_claims.exp > access_claims.exp);
    }
}
