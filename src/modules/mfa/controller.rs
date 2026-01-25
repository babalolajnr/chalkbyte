use chalkbyte_core::AppError;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use crate::validator::ValidatedJson;
use axum::Json;
use axum::extract::State;
use tracing::instrument;

use super::model::{
    DisableMfaRequest, EnableMfaResponse, MessageResponse, MfaStatusResponse,
    RegenerateMfaRecoveryCodesResponse, VerifyMfaRequest,
};
use super::service::MfaService;

/// Get MFA enrollment status
#[utoipa::path(
    get,
    path = "/api/mfa/status",
    summary = "Get MFA status",
    responses(
        (status = 200, description = "MFA status retrieved", body = MfaStatusResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "MFA",
    security(("bearer_auth" = []))
)]
#[instrument]
pub async fn get_mfa_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<MfaStatusResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;
    let status = MfaService::get_mfa_status(&state.db, user_id).await?;
    Ok(Json(status))
}

/// Enable MFA and get QR code
#[utoipa::path(
    post,
    path = "/api/mfa/enable",
    summary = "Enable MFA",
    responses(
        (status = 200, description = "MFA secret generated, awaiting verification", body = EnableMfaResponse),
        (status = 400, description = "MFA already enabled"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "MFA",
    security(("bearer_auth" = []))
)]
#[instrument]
#[axum::debug_handler]
pub async fn enable_mfa(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<EnableMfaResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;
    let response = MfaService::generate_mfa_secret(&state.db, user_id, &auth_user.0.email).await?;
    Ok(Json(response))
}

/// Verify MFA setup with TOTP code
#[utoipa::path(
    post,
    path = "/api/mfa/verify",
    summary = "Verify MFA code",
    request_body = VerifyMfaRequest,
    responses(
        (status = 200, description = "MFA enabled successfully", body = RegenerateMfaRecoveryCodesResponse),
        (status = 400, description = "Invalid TOTP code or MFA not initialized"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "MFA",
    security(("bearer_auth" = []))
)]
#[instrument]
pub async fn verify_mfa(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(dto): ValidatedJson<VerifyMfaRequest>,
) -> Result<Json<RegenerateMfaRecoveryCodesResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;
    let response = MfaService::verify_and_enable_mfa(&state.db, user_id, &dto.code).await?;
    Ok(Json(response))
}

/// Disable MFA with password confirmation
#[utoipa::path(
    post,
    path = "/api/mfa/disable",
    summary = "Disable MFA",
    request_body = DisableMfaRequest,
    responses(
        (status = 200, description = "MFA disabled successfully", body = MessageResponse),
        (status = 400, description = "Invalid password or MFA not enabled"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "MFA",
    security(("bearer_auth" = []))
)]
#[instrument]
pub async fn disable_mfa(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(dto): ValidatedJson<DisableMfaRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;
    MfaService::disable_mfa(&state.db, user_id, &dto.password).await?;
    Ok(Json(MessageResponse {
        message: "MFA has been disabled successfully".to_string(),
    }))
}

/// Regenerate recovery codes
#[utoipa::path(
    post,
    path = "/api/mfa/recovery-codes/regenerate",
    summary = "Regenerate recovery codes",
    responses(
        (status = 200, description = "Recovery codes regenerated", body = RegenerateMfaRecoveryCodesResponse),
        (status = 400, description = "MFA not enabled"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "MFA",
    security(("bearer_auth" = []))
)]
#[instrument]
pub async fn regenerate_recovery_codes(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<RegenerateMfaRecoveryCodesResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;
    let response = MfaService::regenerate_recovery_codes(&state.db, user_id).await?;
    Ok(Json(response))
}
