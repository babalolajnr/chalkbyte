use crate::state::AppState;
use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;
use axum::Json;
use axum::extract::State;

use axum::response::IntoResponse;
use tracing::instrument;
use utoipa::ToSchema;

use super::model::{
    ForgotPasswordRequest, LoginRequest, LoginResponse, MessageResponse, MfaRecoveryLoginRequest,
    MfaRequiredResponse, MfaVerifyLoginRequest, RefreshTokenRequest, ResetPasswordRequest,
};
use super::service::AuthService;
use crate::middleware::auth::AuthUser;
use uuid::Uuid;

#[derive(ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Login and receive JWT token or MFA challenge
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 200, description = "MFA required", body = MfaRequiredResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn login_user(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<LoginRequest>,
) -> Result<axum::response::Response, AppError> {
    match AuthService::login_user(&state.db, dto, &state.jwt_config).await? {
        Ok(login_response) => Ok(Json(login_response).into_response()),
        Err(mfa_required) => Ok(Json(mfa_required).into_response()),
    }
}

/// Verify MFA code and complete login
#[utoipa::path(
    post,
    path = "/api/auth/mfa/verify",
    request_body = MfaVerifyLoginRequest,
    responses(
        (status = 200, description = "MFA verification successful", body = LoginResponse),
        (status = 401, description = "Invalid MFA code or temp token", body = ErrorResponse),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn verify_mfa_login(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<MfaVerifyLoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = AuthService::verify_mfa_login(&state.db, dto, &state.jwt_config).await?;
    Ok(Json(response))
}

/// Use recovery code to complete login
#[utoipa::path(
    post,
    path = "/api/auth/mfa/recovery",
    request_body = MfaRecoveryLoginRequest,
    responses(
        (status = 200, description = "Recovery code verification successful", body = LoginResponse),
        (status = 401, description = "Invalid recovery code or temp token", body = ErrorResponse),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn verify_mfa_recovery_login(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<MfaRecoveryLoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response =
        AuthService::verify_mfa_recovery_login(&state.db, dto, &state.jwt_config).await?;
    Ok(Json(response))
}

/// Request password reset email
#[utoipa::path(
    post,
    path = "/api/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent if account exists", body = MessageResponse),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn forgot_password(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<ForgotPasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    AuthService::forgot_password(&state.db, dto, &state.email_config).await?;
    Ok(Json(MessageResponse {
        message: "If an account exists with that email, a password reset link has been sent."
            .to_string(),
    }))
}

/// Reset password using token
#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = MessageResponse),
        (status = 400, description = "Bad request - invalid or expired token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn reset_password(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    AuthService::reset_password(&state.db, dto, &state.email_config).await?;
    Ok(Json(MessageResponse {
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}

/// Refresh access token using refresh token
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = LoginResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn refresh_token(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = AuthService::refresh_access_token(&state.db, dto, &state.jwt_config).await?;
    Ok(Json(response))
}

/// Logout and revoke all refresh tokens
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication",
    security(("bearer_auth" = []))
)]
#[instrument]
pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<MessageResponse>, AppError> {
    let user_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid token".to_string()))?;

    AuthService::revoke_all_refresh_tokens(&state.db, user_id).await?;
    Ok(Json(MessageResponse {
        message: "Logged out successfully. All refresh tokens have been revoked.".to_string(),
    }))
}
