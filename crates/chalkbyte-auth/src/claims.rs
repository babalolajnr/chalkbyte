//! JWT claim structures for authentication tokens.
//!
//! This module contains all JWT claim structures used in the Chalkbyte authentication system:
//!
//! - [`Claims`]: Access token claims with full user information
//! - [`RefreshTokenClaims`]: Refresh token claims for token renewal
//! - [`MfaTempClaims`]: Temporary token claims for MFA verification flow

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// JWT claims for access tokens.
///
/// These claims are embedded in access tokens and provide all necessary
/// information for authentication and authorization without database lookups.
///
/// # Fields
///
/// - `sub`: User ID (subject)
/// - `email`: User's email address
/// - `school_id`: School scope (None for system admins)
/// - `role_ids`: List of assigned role UUIDs
/// - `permissions`: List of permission strings derived from roles
/// - `exp`: Token expiration timestamp
/// - `iat`: Token issued-at timestamp
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    /// User ID (subject claim)
    pub sub: String,
    /// User's email address
    pub email: String,
    /// User's school_id for scoping (None for system admins)
    pub school_id: Option<Uuid>,
    /// Role IDs assigned to the user
    pub role_ids: Vec<Uuid>,
    /// Permission names granted to the user (derived from roles)
    pub permissions: Vec<String>,
    /// Token expiration timestamp (Unix timestamp)
    pub exp: usize,
    /// Token issued-at timestamp (Unix timestamp)
    pub iat: usize,
}

/// JWT claims for MFA temporary tokens.
///
/// Issued after successful password authentication when MFA is enabled.
/// This token has a short lifetime (10 minutes) and can only be used
/// to complete the MFA verification flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaTempClaims {
    /// User ID (subject claim)
    pub sub: String,
    /// User's email address
    pub email: String,
    /// Flag indicating MFA verification is pending
    pub mfa_pending: bool,
    /// Token expiration timestamp (Unix timestamp)
    pub exp: usize,
    /// Token issued-at timestamp (Unix timestamp)
    pub iat: usize,
}

/// JWT claims for refresh tokens.
///
/// Refresh tokens are long-lived and used to obtain new access tokens
/// without requiring the user to re-authenticate with their password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// User ID (subject claim)
    pub sub: String,
    /// User's email address
    pub email: String,
    /// Token expiration timestamp (Unix timestamp)
    pub exp: usize,
    /// Token issued-at timestamp (Unix timestamp)
    pub iat: usize,
    /// Unique token identifier (JWT ID) to ensure token uniqueness
    pub jti: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_claims_with_school_id() {
        let school_id = Uuid::new_v4();
        let claims = Claims {
            sub: "user-123".to_string(),
            email: "user@school.com".to_string(),
            school_id: Some(school_id),
            role_ids: vec![Uuid::new_v4()],
            permissions: vec!["users:read".to_string(), "users:create".to_string()],
            exp: 1234567890,
            iat: 1234567800,
        };
        assert_eq!(claims.school_id, Some(school_id));
        assert_eq!(claims.permissions.len(), 2);
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
    fn test_refresh_token_claims_serialize() {
        let claims = RefreshTokenClaims {
            sub: "user-123".to_string(),
            email: "refresh@test.com".to_string(),
            exp: 1234567890,
            iat: 1234567800,
            jti: "test-jti-123".to_string(),
        };
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains(r#""sub":"user-123""#));
        assert!(serialized.contains(r#""email":"refresh@test.com""#));
    }

    #[test]
    fn test_refresh_token_claims_clone() {
        let claims = RefreshTokenClaims {
            sub: "user-456".to_string(),
            email: "clone@test.com".to_string(),
            exp: 1234567890,
            iat: 1234567800,
            jti: "test-jti-456".to_string(),
        };
        let cloned = claims.clone();
        assert_eq!(claims.sub, cloned.sub);
        assert_eq!(claims.email, cloned.email);
    }

    #[test]
    fn test_mfa_temp_claims_clone() {
        let claims = MfaTempClaims {
            sub: "user-789".to_string(),
            email: "mfa-clone@test.com".to_string(),
            mfa_pending: true,
            exp: 1234567890,
            iat: 1234567800,
        };
        let cloned = claims.clone();
        assert_eq!(claims.sub, cloned.sub);
        assert!(cloned.mfa_pending);
    }
}
