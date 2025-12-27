use crate::middleware::auth::AuthUser;
use crate::middleware::role::{get_user_id_from_auth, is_admin, is_system_admin};
use crate::modules::auth::controller::ErrorResponse;
use crate::modules::users::model::{
    ChangePasswordDto, CreateUserDto, PaginatedUsersResponse, UpdateProfileDto, User,
    UserFilterParams, UserWithSchool, system_roles,
};
use crate::modules::users::service::UserService;
use crate::state::AppState;
use crate::utils::errors::AppError;
use axum::{
    Json,
    extract::{Query, State},
};
use serde::Serialize;
use tracing::{debug, info, instrument, warn};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Serialize, ToSchema)]
pub struct ProfileResponse {
    #[serde(flatten)]
    pub info: UserWithSchool,
}

/// Create a new user (requires admin or system_admin role)
#[utoipa::path(
    post,
    path = "/api/users",
    request_body = CreateUserDto,
    responses(
        (status = 200, description = "User created successfully", body = User),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - Admin or System Admin only", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument(skip(state, auth_user, dto), fields(
    user.id = %auth_user.0.sub,
    new_user.email = %dto.email
))]
pub async fn create_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(mut dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    debug!(email = %dto.email, "Processing user creation request");

    let user_id = get_user_id_from_auth(&auth_user)?;

    // Validate DTO
    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation error: {}", e)))?;

    // Check if requester is an admin (system admin or school admin)
    let is_sys_admin = is_system_admin(&state.db, user_id).await?;
    let is_any_admin = is_admin(&state.db, user_id).await?;

    if !is_any_admin {
        warn!(
            user.id = %auth_user.0.sub,
            "Unauthorized attempt to create user"
        );
        return Err(AppError::forbidden(
            "Only admins can create users".to_string(),
        ));
    }

    // School admins can only create users for their school
    if !is_sys_admin {
        // Get requester's school_id from database
        let requester = UserService::get_user(&state.db, user_id).await?;

        let requester_school_id = requester
            .school_id
            .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

        // Ensure the new user is assigned to the same school
        dto.school_id = Some(requester_school_id);

        // School admins cannot create system admins
        if dto.role_ids.contains(&system_roles::SYSTEM_ADMIN) {
            warn!(
                user.id = %auth_user.0.sub,
                "School admin attempted to create system admin"
            );
            return Err(AppError::forbidden(
                "School admins cannot create system admins".to_string(),
            ));
        }
    }

    // Validate school_id exists if provided
    if let Some(school_id) = dto.school_id {
        use crate::modules::schools::service::SchoolService;
        SchoolService::get_school_by_id(&state.db, school_id).await?;
    }

    let user = UserService::create_user(&state.db, dto).await?;

    info!(
        created_user.id = %user.id,
        created_user.email = %user.email,
        "User created successfully"
    );

    Ok(Json(user))
}

/// Get all users with pagination and filtering (system admins see all, school admins see their school)
#[utoipa::path(
    get,
    path = "/api/users",
    params(
        ("first_name" = Option<String>, Query, description = "Filter by first name (partial match)"),
        ("last_name" = Option<String>, Query, description = "Filter by last name (partial match)"),
        ("email" = Option<String>, Query, description = "Filter by email (partial match)"),
        ("role" = Option<String>, Query, description = "Filter by role (system_admin, admin, teacher, student)"),
        ("school_id" = Option<String>, Query, description = "Filter by school ID"),
        ("limit" = Option<i64>, Query, description = "Number of items per page (1-100, default: 10)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "Paginated list of users", body = PaginatedUsersResponse),
        (status = 401, description = "Unauthorized - missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Forbidden - Only admins can list users", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument(skip(state, auth_user, filters), fields(
    user.id = %auth_user.0.sub
))]
pub async fn get_users(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(filters): Query<UserFilterParams>,
) -> Result<Json<PaginatedUsersResponse>, AppError> {
    debug!(filters = ?filters, "Fetching users with filters");

    let user_id = get_user_id_from_auth(&auth_user)?;

    // Check if requester is an admin
    let is_sys_admin = is_system_admin(&state.db, user_id).await?;
    let is_any_admin = is_admin(&state.db, user_id).await?;

    if !is_any_admin {
        warn!(
            user.id = %auth_user.0.sub,
            "Unauthorized role attempted to list users"
        );
        return Err(AppError::forbidden(
            "Only admins can list users".to_string(),
        ));
    }

    let school_id_filter = if is_sys_admin {
        None
    } else {
        let requester = UserService::get_user(&state.db, user_id).await?;

        let school_id = requester
            .school_id
            .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

        Some(school_id)
    };

    let response = UserService::get_users_paginated(&state.db, filters, school_id_filter).await?;

    debug!(
        total = %response.meta.total,
        returned = %response.data.len(),
        "Users fetched successfully"
    );

    Ok(Json(response))
}

/// Get current user profile from JWT token
#[utoipa::path(
    get,
    path = "/api/users/profile",
    responses(
        (status = 200, description = "User profile", body = UserWithSchool),
        (status = 401, description = "Unauthorized - missing or invalid token", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument(skip(state, auth_user), fields(user.id = %auth_user.0.sub))]
pub async fn get_profile(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserWithSchool>, AppError> {
    debug!("Fetching user profile");

    let user = UserService::get_user_with_school(
        &state.db,
        uuid::Uuid::parse_str(&auth_user.0.sub)
            .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?,
    )
    .await?;

    Ok(Json(user))
}

/// Update current user profile (name only)
#[utoipa::path(
    put,
    path = "/api/users/profile",
    request_body = UpdateProfileDto,
    responses(
        (status = 200, description = "Profile updated successfully", body = UserWithSchool),
        (status = 400, description = "Bad request - validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - missing or invalid token", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument(skip(state, auth_user, dto), fields(
    user.id = %auth_user.0.sub,
    update.first_name = ?dto.first_name,
    update.last_name = ?dto.last_name
))]
pub async fn update_profile(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<UpdateProfileDto>,
) -> Result<Json<UserWithSchool>, AppError> {
    debug!("Processing profile update request");

    dto.validate()
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Validation error: {}", e)))?;

    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?;

    UserService::update_profile(&state.db, user_id, dto).await?;

    let user = UserService::get_user_with_school(&state.db, user_id).await?;

    info!(user.id = %user_id, "Profile updated successfully");

    Ok(Json(user))
}

/// Change current user password
#[utoipa::path(
    post,
    path = "/api/users/profile/change-password",
    request_body = ChangePasswordDto,
    responses(
        (status = 200, description = "Password changed successfully", body = inline(serde_json::Value)),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - current password incorrect or invalid token", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
#[instrument(skip(state, auth_user, dto), fields(user.id = %auth_user.0.sub))]
pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<ChangePasswordDto>,
) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Processing password change request");

    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation error: {}", e)))?;

    let user_id = uuid::Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?;

    UserService::change_password(&state.db, user_id, dto).await?;

    info!(user.id = %user_id, "Password changed successfully");

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}
