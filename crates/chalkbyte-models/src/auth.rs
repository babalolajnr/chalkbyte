//! Authentication domain models and DTOs.
//!
//! This module contains all data structures used for authentication operations,
//! including login/logout requests and responses, MFA verification,
//! and password reset flows.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::roles::{Permission, RoleWithPermissions};
use crate::users::{BranchInfo, LevelInfo, SchoolInfo};

// Re-export JWT claim types from chalkbyte-auth for backward compatibility
pub use chalkbyte_auth::{Claims, MfaTempClaims, RefreshTokenClaims};

/// Login request with email and password.
///
/// Used for the initial authentication step. If MFA is enabled,
/// successful authentication returns an [`MfaRequiredResponse`] instead
/// of a [`LoginResponse`].
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    #[schema(example = "password123")]
    pub password: String,
}

/// User info returned in login response with joined relations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginUser {
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub grade_level: Option<String>,
    pub school: Option<SchoolInfo>,
    pub level: Option<LevelInfo>,
    pub branch: Option<BranchInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Successful login response with tokens and user information.
///
/// Returned after successful authentication (including MFA if enabled).
/// Contains both access and refresh tokens, along with full user details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: LoginUser,
    pub roles: Vec<RoleWithPermissions>,
    pub permissions: Vec<Permission>,
}

/// Response indicating MFA verification is required.
///
/// Returned when the user has MFA enabled. The `temp_token` must be
/// submitted along with a TOTP code to complete authentication.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MfaRequiredResponse {
    pub mfa_required: bool,
    pub temp_token: String,
}

/// MFA verification request with TOTP code.
///
/// Submit this with the temp token from [`MfaRequiredResponse`] to
/// complete MFA verification and receive a full [`LoginResponse`].
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct MfaVerifyLoginRequest {
    #[validate(length(min = 1))]
    pub temp_token: String,
    #[validate(length(equal = 6))]
    #[schema(example = "123456")]
    pub code: String,
}

/// MFA recovery code login request.
///
/// Used when the user doesn't have access to their authenticator app.
/// Each recovery code can only be used once.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct MfaRecoveryLoginRequest {
    #[validate(length(min = 1))]
    pub temp_token: String,
    #[validate(length(equal = 8))]
    #[schema(example = "ABCD1234")]
    pub recovery_code: String,
}

/// Request to refresh an access token.
///
/// Submit a valid refresh token to receive a new access token
/// without re-authenticating.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

/// Forgot password request to initiate password reset.
///
/// Submitting this request sends a password reset email to the user
/// if the email exists in the system.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    #[schema(example = "user@example.com")]
    pub email: String,
}

/// Reset password request to complete password reset.
///
/// Submit the token received via email along with the new password
/// to complete the password reset process.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8))]
    #[schema(example = "newPassword123")]
    pub new_password: String,
}

/// Generic success message response.
///
/// Used for operations that don't return specific data,
/// such as logout or password reset initiation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_claims_serialize() {
        let claims = Claims {
            sub: "user-id-123".to_string(),
            email: "test@example.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec!["users:read".to_string()],
            exp: 1234567890,
            iat: 1234567800,
        };
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains(r#""sub":"user-id-123""#));
        assert!(serialized.contains(r#""email":"test@example.com""#));
    }

    #[test]
    fn test_claims_deserialize() {
        let json = r#"{"sub":"user-id-456","email":"user@test.com","school_id":null,"role_ids":[],"permissions":[],"exp":9999999999,"iat":9999999900}"#;
        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user-id-456");
        assert_eq!(claims.email, "user@test.com");
        assert_eq!(claims.exp, 9999999999);
        assert_eq!(claims.iat, 9999999900);
    }

    #[test]
    fn test_claims_clone() {
        let claims = Claims {
            sub: "user-id-789".to_string(),
            email: "clone@example.com".to_string(),
            school_id: None,
            role_ids: vec![],
            permissions: vec![],
            exp: 1234567890,
            iat: 1234567800,
        };
        let cloned = claims.clone();
        assert_eq!(claims.sub, cloned.sub);
        assert_eq!(claims.email, cloned.email);
    }

    #[test]
    fn test_login_request_valid_email() {
        let request = LoginRequest {
            email: "valid@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_login_request_invalid_email() {
        let request = LoginRequest {
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_login_request_empty_password() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_mfa_verify_login_request_valid() {
        let request = MfaVerifyLoginRequest {
            temp_token: "temp-token-value".to_string(),
            code: "123456".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_mfa_verify_login_request_code_too_short() {
        let request = MfaVerifyLoginRequest {
            temp_token: "temp-token-value".to_string(),
            code: "12345".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_mfa_verify_login_request_code_too_long() {
        let request = MfaVerifyLoginRequest {
            temp_token: "temp-token-value".to_string(),
            code: "1234567".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_mfa_recovery_login_request_valid() {
        let request = MfaRecoveryLoginRequest {
            temp_token: "temp-token-value".to_string(),
            recovery_code: "ABCD1234".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_mfa_recovery_login_request_code_too_short() {
        let request = MfaRecoveryLoginRequest {
            temp_token: "temp-token-value".to_string(),
            recovery_code: "ABC123".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_refresh_token_request_valid() {
        let request = RefreshTokenRequest {
            refresh_token: "valid-refresh-token".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_refresh_token_request_empty() {
        let request = RefreshTokenRequest {
            refresh_token: "".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_refresh_token_claims_serialize() {
        let claims = RefreshTokenClaims {
            sub: "user-123".to_string(),
            email: "refresh@test.com".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains(r#""sub":"user-123""#));
    }

    #[test]
    fn test_reset_password_request_valid() {
        let request = ResetPasswordRequest {
            token: "valid-reset-token".to_string(),
            new_password: "newPassword123".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_reset_password_request_password_too_short() {
        let request = ResetPasswordRequest {
            token: "valid-reset-token".to_string(),
            new_password: "short".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_reset_password_request_empty_token() {
        let request = ResetPasswordRequest {
            token: "".to_string(),
            new_password: "validPassword123".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_mfa_temp_claims_serialize() {
        let claims = MfaTempClaims {
            sub: "user-mfa-123".to_string(),
            email: "mfa@test.com".to_string(),
            mfa_pending: true,
            exp: 1234567890,
            iat: 1234567800,
        };
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains(r#""mfa_pending":true"#));
        assert!(serialized.contains(r#""email":"mfa@test.com""#));
    }

    #[test]
    fn test_login_request_special_characters_email() {
        let request = LoginRequest {
            email: "test+tag@example.co.uk".to_string(),
            password: "password123".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_reset_password_minimum_length() {
        let request = ResetPasswordRequest {
            token: "token".to_string(),
            new_password: "12345678".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_mfa_required_response_serialize() {
        let response = MfaRequiredResponse {
            mfa_required: true,
            temp_token: "temporary-token".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains(r#""mfa_required":true"#));
        assert!(serialized.contains(r#""temp_token":"temporary-token""#));
    }

    #[test]
    fn test_message_response_serialize() {
        let response = MessageResponse {
            message: "Operation successful".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains(r#""message":"Operation successful""#));
    }
}
