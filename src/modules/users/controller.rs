use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::auth::controller::ErrorResponse;
use crate::modules::users::model::{CreateUserDto, User};
use crate::modules::users::service::UserService;
use crate::utils::errors::AppError;
use axum::{Json, extract::State};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ProfileResponse {
    pub user_id: String,
    pub email: String,
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/api/users",
    request_body = CreateUserDto,
    responses(
        (status = 200, description = "User created successfully", body = User),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Users"
)]
#[instrument]
pub async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}

/// Get all users (requires authentication)
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of users", body = Vec<User>),
        (status = 401, description = "Unauthorized - missing or invalid token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument]
pub async fn get_users(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    let users = UserService::get_users(&state.db).await?;
    Ok(Json(users))
}

/// Get current user profile from JWT token
#[utoipa::path(
    get,
    path = "/api/users/profile",
    responses(
        (status = 200, description = "User profile", body = ProfileResponse),
        (status = 401, description = "Unauthorized - missing or invalid token", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument]
pub async fn get_profile(auth_user: AuthUser) -> Result<Json<ProfileResponse>, AppError> {
    Ok(Json(ProfileResponse {
        user_id: auth_user.0.sub,
        email: auth_user.0.email,
    }))
}
