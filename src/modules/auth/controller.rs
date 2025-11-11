use crate::db::AppState;
use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::instrument;
use utoipa::ToSchema;

use super::model::{LoginRequest, LoginResponse, RegisterRequestDto};
use super::service::AuthService;
use crate::modules::users::model::User;

#[derive(ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Register a new user
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequestDto,
    responses(
        (status = 201, description = "User registered successfully", body = User),
        (status = 400, description = "Bad request - validation error or email already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
#[instrument]
pub async fn register_user(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<RegisterRequestDto>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = AuthService::register_user(&state.db, dto).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// Login and receive JWT token
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
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
) -> Result<Json<LoginResponse>, AppError> {
    let response = AuthService::login_user(&state.db, dto, &state.jwt_config).await?;
    Ok(Json(response))
}
