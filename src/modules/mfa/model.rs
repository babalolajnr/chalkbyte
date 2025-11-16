use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MfaLoginRequest {
    #[validate(length(equal = 6))]
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MfaRecoveryLoginRequest {
    #[validate(length(equal = 8))]
    #[schema(example = "ABCD1234")]
    pub recovery_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MfaRequiredResponse {
    pub mfa_required: bool,
    pub temp_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MfaTempClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub mfa_pending: bool,
    pub exp: usize,
    pub iat: usize,
}

#[derive(sqlx::FromRow)]
pub struct MfaRecoveryCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub used: bool,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
