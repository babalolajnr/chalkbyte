use crate::db::AppState;
use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::instrument;

use super::model::{LoginRequest, LoginResponse, RegisterRequestDto};
use super::service::AuthService;
use crate::modules::users::model::User;

#[instrument]
pub async fn register_user(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<RegisterRequestDto>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = AuthService::register_user(&state.db, dto).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

#[instrument]
pub async fn login_user(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let response = AuthService::login_user(&state.db, dto, &state.jwt_config).await?;
    Ok(Json(response))
}
