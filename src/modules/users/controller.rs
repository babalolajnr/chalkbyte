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
#[instrument]
pub async fn create_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(mut dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    use crate::modules::users::model::UserRole;
    
    // Parse role from string to UserRole for validation
    let requester_role = match auth_user.0.role.as_str() {
        "system_admin" => UserRole::SystemAdmin,
        "admin" => UserRole::Admin,
        "teacher" => UserRole::Teacher,
        "student" => UserRole::Student,
        _ => return Err(AppError::forbidden("Invalid role".to_string())),
    };
    
    // Only system_admin and admin can create users
    if requester_role != UserRole::SystemAdmin && requester_role != UserRole::Admin {
        return Err(AppError::forbidden("Only admins can create users".to_string()));
    }
    
    // School admins can only create users for their school
    if requester_role == UserRole::Admin {
        // Get requester's school_id from database
        let requester = UserService::get_user(&state.db, 
            uuid::Uuid::parse_str(&auth_user.0.sub)
                .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?
        ).await?;
        
        let requester_school_id = requester.school_id
            .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;
        
        // Ensure the new user is assigned to the same school
        dto.school_id = Some(requester_school_id);
        
        // School admins cannot create system_admin or admin for other schools
        if let Some(ref role) = dto.role {
            if role == &UserRole::SystemAdmin {
                return Err(AppError::forbidden("School admins cannot create system admins".to_string()));
            }
        }
    }
    
    // System admins can create any user
    // Validate school_id exists if provided
    if let Some(school_id) = dto.school_id {
        use crate::modules::schools::service::SchoolService;
        SchoolService::get_school_by_id(&state.db, school_id).await?;
    }
    
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}

/// Get all users (system admins see all, school admins see their school)
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
    auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    use crate::modules::users::model::UserRole;
    
    let requester_role = match auth_user.0.role.as_str() {
        "system_admin" => UserRole::SystemAdmin,
        "admin" => UserRole::Admin,
        _ => return Err(AppError::forbidden("Only admins can list users".to_string())),
    };
    
    let users = if requester_role == UserRole::SystemAdmin {
        // System admin can see all users
        UserService::get_users(&state.db).await?
    } else {
        // School admin can only see users from their school
        let requester = UserService::get_user(&state.db, 
            uuid::Uuid::parse_str(&auth_user.0.sub)
                .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?
        ).await?;
        
        let school_id = requester.school_id
            .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;
        
        UserService::get_users_by_school(&state.db, school_id).await?
    };
    
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
