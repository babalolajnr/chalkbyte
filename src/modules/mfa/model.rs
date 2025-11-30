use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, ToSchema)]
pub struct EnableMfaResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub qr_code_base64: String,
    pub manual_entry_key: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyMfaRequest {
    #[validate(length(equal = 6))]
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DisableMfaRequest {
    #[validate(length(min = 8))]
    #[schema(example = "password123")]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MfaStatusResponse {
    pub mfa_enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegenerateMfaRecoveryCodesResponse {
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
