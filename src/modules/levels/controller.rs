use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

use crate::middleware::auth::AuthUser;
use crate::middleware::role::{get_user_id_from_auth, is_admin};
use crate::modules::levels::model::{
    AssignStudentsToLevelDto, BulkAssignResponse, CreateLevelDto, Level, LevelFilterParams,
    LevelWithStats, MoveStudentToLevelDto, PaginatedLevelsResponse, UpdateLevelDto,
};
use crate::modules::levels::service::LevelService;
use crate::modules::users::model::User;
use crate::state::AppState;
use crate::utils::auth_helpers::get_admin_school_id;
use crate::utils::errors::AppError;

/// Helper to verify admin access
async fn require_admin_access(db: &sqlx::PgPool, auth_user: &AuthUser) -> Result<Uuid, AppError> {
    let user_id = get_user_id_from_auth(auth_user)?;

    if !is_admin(db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can perform this action".to_string(),
        ));
    }

    get_admin_school_id(db, auth_user).await
}

#[utoipa::path(
    post,
    path = "/api/levels",
    request_body = CreateLevelDto,
    responses(
        (status = 201, description = "Level created successfully", body = Level),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let level = LevelService::create_level(&state.db, school_id, dto).await?;

    Ok((StatusCode::CREATED, Json(level)))
}

#[utoipa::path(
    get,
    path = "/api/levels",
    params(LevelFilterParams),
    responses(
        (status = 200, description = "List of levels", body = PaginatedLevelsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_levels(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(filters): Query<LevelFilterParams>,
) -> Result<Json<PaginatedLevelsResponse>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let levels = LevelService::get_levels_by_school(&state.db, school_id, filters).await?;

    Ok(Json(levels))
}

#[utoipa::path(
    get,
    path = "/api/levels/{id}",
    params(
        ("id" = Uuid, Path, description = "Level ID")
    ),
    responses(
        (status = 200, description = "Level details", body = LevelWithStats),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_level_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LevelWithStats>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let level = LevelService::get_level_by_id(&state.db, id, school_id).await?;

    Ok(Json(level))
}

#[utoipa::path(
    put,
    path = "/api/levels/{id}",
    params(
        ("id" = Uuid, Path, description = "Level ID")
    ),
    request_body = UpdateLevelDto,
    responses(
        (status = 200, description = "Level updated successfully", body = Level),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateLevelDto>,
) -> Result<Json<Level>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let level = LevelService::update_level(&state.db, id, school_id, dto).await?;

    Ok(Json(level))
}

#[utoipa::path(
    delete,
    path = "/api/levels/{id}",
    params(
        ("id" = Uuid, Path, description = "Level ID")
    ),
    responses(
        (status = 204, description = "Level deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    LevelService::delete_level(&state.db, id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/levels/{id}/students",
    params(
        ("id" = Uuid, Path, description = "Level ID")
    ),
    request_body = AssignStudentsToLevelDto,
    responses(
        (status = 200, description = "Students assigned to level", body = BulkAssignResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn assign_students_to_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignStudentsToLevelDto>,
) -> Result<Json<BulkAssignResponse>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let response = LevelService::assign_students_to_level(&state.db, id, school_id, dto).await?;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/levels/{id}/students",
    params(
        ("id" = Uuid, Path, description = "Level ID")
    ),
    responses(
        (status = 200, description = "List of students in level", body = Vec<User>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_students_in_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<User>>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let students = LevelService::get_students_in_level(&state.db, id, school_id).await?;

    Ok(Json(students))
}

#[utoipa::path(
    patch,
    path = "/api/levels/students/{student_id}/move",
    params(
        ("student_id" = Uuid, Path, description = "Student ID")
    ),
    request_body = MoveStudentToLevelDto,
    responses(
        (status = 204, description = "Student moved successfully"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Student or level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn move_student_to_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
    Json(dto): Json<MoveStudentToLevelDto>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    LevelService::move_student_to_level(&state.db, student_id, school_id, dto).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/levels/students/{student_id}",
    params(
        ("student_id" = Uuid, Path, description = "Student ID")
    ),
    responses(
        (status = 204, description = "Student removed from level"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Student not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn remove_student_from_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    LevelService::remove_student_from_level(&state.db, student_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
