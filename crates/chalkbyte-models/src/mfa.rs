//! MFA (Multi-Factor Authentication) domain models and DTOs.
//!
//! This module contains all data structures related to multi-factor authentication,
//! including MFA setup, verification, and recovery operations.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Response when enabling MFA for a user.
///
/// Contains the TOTP secret and QR code for scanning with an authenticator app.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnableMfaResponse {
    /// The TOTP secret key (base32 encoded)
    pub secret: String,
    /// The otpauth:// URL for the QR code
    pub qr_code_url: String,
    /// Base64 encoded PNG image of the QR code
    pub qr_code_base64: String,
    /// Manual entry key for authenticator apps that don't support QR codes
    pub manual_entry_key: String,
}

/// Request to verify MFA setup with a TOTP code.
///
/// Used to confirm that the user has correctly set up their authenticator app
/// by providing a valid TOTP code.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyMfaRequest {
    #[validate(length(equal = 6))]
    #[schema(example = "123456")]
    pub code: String,
}

/// Request to disable MFA for the current user.
///
/// Requires the user's password for security verification.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DisableMfaRequest {
    #[validate(length(min = 8))]
    #[schema(example = "password123")]
    pub password: String,
}

/// Response containing the current MFA status for a user.
#[derive(Debug, Serialize, ToSchema)]
pub struct MfaStatusResponse {
    /// Whether MFA is currently enabled for the user
    pub mfa_enabled: bool,
}

/// Response containing newly generated recovery codes.
///
/// Recovery codes are single-use codes that can be used to access the account
/// if the authenticator app is unavailable.
#[derive(Debug, Serialize, ToSchema)]
pub struct RegenerateMfaRecoveryCodesResponse {
    /// List of recovery codes (typically 10 codes, each 8 characters)
    pub recovery_codes: Vec<String>,
}

/// Generic success message response for MFA operations.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_mfa_request_valid() {
        let request = VerifyMfaRequest {
            code: "123456".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_verify_mfa_request_code_too_short() {
        let request = VerifyMfaRequest {
            code: "12345".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_verify_mfa_request_code_too_long() {
        let request = VerifyMfaRequest {
            code: "1234567".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_disable_mfa_request_valid() {
        let request = DisableMfaRequest {
            password: "password123".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_disable_mfa_request_password_too_short() {
        let request = DisableMfaRequest {
            password: "short".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_enable_mfa_response_serialize() {
        let response = EnableMfaResponse {
            secret: "JBSWY3DPEHPK3PXP".to_string(),
            qr_code_url:
                "otpauth://totp/Chalkbyte:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Chalkbyte"
                    .to_string(),
            qr_code_base64: "iVBORw0KGgo=".to_string(),
            manual_entry_key: "JBSW Y3DP EHPK 3PXP".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains(r#""secret":"JBSWY3DPEHPK3PXP""#));
        assert!(serialized.contains(r#""qr_code_url":"#));
    }

    #[test]
    fn test_mfa_status_response_serialize() {
        let enabled = MfaStatusResponse { mfa_enabled: true };
        let disabled = MfaStatusResponse { mfa_enabled: false };

        let enabled_json = serde_json::to_string(&enabled).unwrap();
        let disabled_json = serde_json::to_string(&disabled).unwrap();

        assert!(enabled_json.contains(r#""mfa_enabled":true"#));
        assert!(disabled_json.contains(r#""mfa_enabled":false"#));
    }

    #[test]
    fn test_regenerate_mfa_recovery_codes_response_serialize() {
        let response = RegenerateMfaRecoveryCodesResponse {
            recovery_codes: vec![
                "ABCD1234".to_string(),
                "EFGH5678".to_string(),
                "IJKL9012".to_string(),
            ],
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains(r#""recovery_codes":["ABCD1234","EFGH5678","IJKL9012"]"#));
    }

    #[test]
    fn test_message_response_serialize() {
        let response = MessageResponse {
            message: "MFA successfully enabled".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains(r#""message":"MFA successfully enabled""#));
    }
}
