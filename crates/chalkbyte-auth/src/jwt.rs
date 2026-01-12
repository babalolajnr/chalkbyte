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
//! use chalkbyte_auth::{create_access_token, verify_token};
//! use chalkbyte_config::JwtConfig;
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

use chalkbyte_config::JwtConfig;
use chalkbyte_core::AppError;

use crate::claims::{Claims, MfaTempClaims, RefreshTokenClaims};

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
        jti: Uuid::new_v4().to_string(),
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
            secret: "test-secret-key-at-least-32-characters-long".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        }
    }

    #[test]
    fn test_create_access_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let school_id = Uuid::new_v4();

        let result = create_access_token(
            user_id,
            "test@example.com",
            Some(school_id),
            vec![Uuid::new_v4()],
            vec!["users:read".to_string()],
            &config,
        );

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let school_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let token = create_access_token(
            user_id,
            "test@example.com",
            Some(school_id),
            vec![role_id],
            vec!["users:read".to_string()],
            &config,
        )
        .unwrap();

        let claims = verify_token(&token, &config).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.school_id, Some(school_id));
        assert_eq!(claims.role_ids, vec![role_id]);
        assert_eq!(claims.permissions, vec!["users:read".to_string()]);
    }

    #[test]
    fn test_verify_token_invalid() {
        let config = get_test_jwt_config();
        let result = verify_token("invalid-token", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let token = create_access_token(user_id, "test@example.com", None, vec![], vec![], &config)
            .unwrap();

        let wrong_config = JwtConfig {
            secret: "different-secret-key-at-least-32-characters".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        };

        let result = verify_token(&token, &wrong_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_refresh_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let result = create_refresh_token(user_id, "test@example.com", &config);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_refresh_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let token = create_refresh_token(user_id, "test@example.com", &config).unwrap();
        let claims = verify_refresh_token(&token, &config).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_create_mfa_temp_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let result = create_mfa_temp_token(user_id, "test@example.com", &config);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_mfa_temp_token_success() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let token = create_mfa_temp_token(user_id, "test@example.com", &config).unwrap();
        let claims = verify_mfa_temp_token(&token, &config).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert!(claims.mfa_pending);
    }

    #[test]
    fn test_verify_mfa_temp_token_invalid() {
        let config = get_test_jwt_config();
        let result = verify_mfa_temp_token("invalid-token", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token_expiry_longer_than_access() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let access_token =
            create_access_token(user_id, "test@example.com", None, vec![], vec![], &config)
                .unwrap();

        let refresh_token = create_refresh_token(user_id, "test@example.com", &config).unwrap();

        let access_claims = verify_token(&access_token, &config).unwrap();
        let refresh_claims = verify_refresh_token(&refresh_token, &config).unwrap();

        assert!(refresh_claims.exp > access_claims.exp);
    }

    #[test]
    fn test_token_with_no_school_id() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();

        let token = create_access_token(
            user_id,
            "sysadmin@example.com",
            None,
            vec![],
            vec!["*".to_string()],
            &config,
        )
        .unwrap();

        let claims = verify_token(&token, &config).unwrap();
        assert!(claims.school_id.is_none());
    }

    #[test]
    fn test_token_with_multiple_permissions() {
        let config = get_test_jwt_config();
        let user_id = Uuid::new_v4();
        let permissions = vec![
            "users:read".to_string(),
            "users:create".to_string(),
            "users:update".to_string(),
            "users:delete".to_string(),
        ];

        let token = create_access_token(
            user_id,
            "admin@example.com",
            Some(Uuid::new_v4()),
            vec![Uuid::new_v4()],
            permissions.clone(),
            &config,
        )
        .unwrap();

        let claims = verify_token(&token, &config).unwrap();
        assert_eq!(claims.permissions, permissions);
    }
}
