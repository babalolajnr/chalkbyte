use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

use chalkbyte_core::AppError;

use crate::middleware::auth::{
    RequireLevelsAssignStudents, RequireLevelsCreate, RequireLevelsDelete, RequireLevelsRead,
    RequireLevelsUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::levels::model::{
    AssignStudentsToLevelDto, BulkAssignResponse, CreateLevelDto, Level, LevelFilterParams,
    LevelWithStats, MoveStudentToLevelDto, PaginatedLevelsResponse, UpdateLevelDto,
};
use crate::modules::levels::service::LevelService;
use crate::modules::users::model::User;
use crate::state::AppState;
use crate::utils::auth_helpers::{get_admin_school_id, get_school_id_for_scoped_operation};

#[utoipa::path(
    post,
    path = "/api/levels",
    request_body = CreateLevelDto,
    responses(
        (status = 201, description = "Level created successfully", body = Level),
        (status = 400, description = "Invalid input or missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires levels:create permission")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_level(
    State(state): State<AppState>,
    RequireLevelsCreate(auth_user): RequireLevelsCreate,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    // Creating requires school_id - system admins must specify it in the DTO
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, dto.school_id).await?;

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
        (status = 400, description = "Missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires levels:read permission")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_levels(
    State(state): State<AppState>,
    RequireLevelsRead(auth_user): RequireLevelsRead,
    Query(filters): Query<LevelFilterParams>,
) -> Result<Json<PaginatedLevelsResponse>, AppError> {
    // Listing requires school_id - system admins must specify it in query params
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, filters.school_id).await?;

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
        (status = 403, description = "Forbidden - requires levels:read permission"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_level_by_id(
    State(state): State<AppState>,
    RequireLevelsRead(auth_user): RequireLevelsRead,
    Path(id): Path<Uuid>,
) -> Result<Json<LevelWithStats>, AppError> {
    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        let level = LevelService::get_level_by_id_no_school_filter(&state.db, id).await?;
        return Ok(Json(level));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:update permission"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_level(
    State(state): State<AppState>,
    RequireLevelsUpdate(auth_user): RequireLevelsUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateLevelDto>,
) -> Result<Json<Level>, AppError> {
    dto.validate()?;

    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        let level = LevelService::update_level_no_school_filter(&state.db, id, dto).await?;
        return Ok(Json(level));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:delete permission"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_level(
    State(state): State<AppState>,
    RequireLevelsDelete(auth_user): RequireLevelsDelete,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        LevelService::delete_level_no_school_filter(&state.db, id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:assign_students permission"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn assign_students_to_level(
    State(state): State<AppState>,
    RequireLevelsAssignStudents(auth_user): RequireLevelsAssignStudents,
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignStudentsToLevelDto>,
) -> Result<Json<BulkAssignResponse>, AppError> {
    dto.validate()?;

    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        let response =
            LevelService::assign_students_to_level_no_school_filter(&state.db, id, dto).await?;
        return Ok(Json(response));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:read permission"),
        (status = 404, description = "Level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_students_in_level(
    State(state): State<AppState>,
    RequireLevelsRead(auth_user): RequireLevelsRead,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<User>>, AppError> {
    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        let students = LevelService::get_students_in_level_no_school_filter(&state.db, id).await?;
        return Ok(Json(students));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:assign_students permission"),
        (status = 404, description = "Student or level not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn move_student_to_level(
    State(state): State<AppState>,
    RequireLevelsAssignStudents(auth_user): RequireLevelsAssignStudents,
    Path(student_id): Path<Uuid>,
    Json(dto): Json<MoveStudentToLevelDto>,
) -> Result<StatusCode, AppError> {
    dto.validate()?;

    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        LevelService::move_student_to_level_no_school_filter(&state.db, student_id, dto).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires levels:assign_students permission"),
        (status = 404, description = "Student not found")
    ),
    tag = "Levels",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn remove_student_from_level(
    State(state): State<AppState>,
    RequireLevelsAssignStudents(auth_user): RequireLevelsAssignStudents,
    Path(student_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // For resource operations, system admins don't need school_id
    if is_system_admin_jwt(&auth_user) {
        LevelService::remove_student_from_level_no_school_filter(&state.db, student_id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    LevelService::remove_student_from_level(&state.db, student_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
