use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{CreateUserDto, User};
use crate::modules::users::service::UserService;
use crate::utils::errors::AppError;
use axum::{Json, extract::State};
use tracing::instrument;

#[instrument]
pub async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}

#[instrument]
pub async fn get_users(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    let users = UserService::get_users(&state.db).await?;
    Ok(Json(users))
}

#[instrument]
pub async fn get_profile(auth_user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({
        "user_id": auth_user.0.sub,
        "email": auth_user.0.email,
    })))
}
