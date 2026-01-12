use chalkbyte_core::AppError;

use crate::middleware::auth::{
    RequireStudentsCreate, RequireStudentsDelete, RequireStudentsRead, RequireStudentsUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::auth::controller::ErrorResponse;
use crate::modules::students::model::{
    CreateStudentDto, PaginatedStudentsResponse, PaginationMeta, QueryParams, Student,
    UpdateStudentDto,
};
use crate::modules::students::service::StudentService;
use crate::state::AppState;
use crate::utils::auth_helpers::{get_admin_school_id, get_school_id_for_scoped_operation};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

#[utoipa::path(
    post,
    path = "/api/students",
    request_body = CreateStudentDto,
    responses(
        (status = 200, description = "Student created successfully", body = Student),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires students:create permission", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument(skip(state))]
pub async fn create_student(
    State(state): State<AppState>,
    RequireStudentsCreate(auth_user): RequireStudentsCreate,
    Json(dto): Json<CreateStudentDto>,
) -> Result<Json<Student>, AppError> {
    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation failed: {}", e)))?;

    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, dto.school_id).await?;

    let student = StudentService::create_student(
        &state.db,
        dto,
        school_id.into_inner(),
        state.cache.as_ref(),
    )
    .await?;
    Ok(Json(student))
}

#[utoipa::path(
    get,
    path = "/api/students",
    params(
        QueryParams
    ),
    responses(
        (status = 200, description = "List of students", body = PaginatedStudentsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires students:read permission", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument(skip(state))]
pub async fn get_students(
    State(state): State<AppState>,
    RequireStudentsRead(auth_user): RequireStudentsRead,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedStudentsResponse>, AppError> {
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, params.school_id).await?;

    let limit = params.limit();
    let offset = params.offset();
    let page = params.page();

    let (students, total) =
        StudentService::get_students_by_school(&state.db, school_id.into_inner(), limit, offset)
            .await?;

    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    let response = PaginatedStudentsResponse {
        data: students,
        meta: PaginationMeta {
            page,
            limit,
            total,
            total_pages,
        },
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/students/{id}",
    params(
        ("id" = Uuid, Path, description = "Student ID")
    ),
    responses(
        (status = 200, description = "Student details", body = Student),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires students:read permission", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument(skip(state))]
pub async fn get_student(
    State(state): State<AppState>,
    RequireStudentsRead(auth_user): RequireStudentsRead,
    Path(id): Path<Uuid>,
) -> Result<Json<Student>, AppError> {
    // System admins can access any student
    if is_system_admin_jwt(&auth_user) {
        let student = StudentService::get_student_by_id_no_school_filter(&state.db, id).await?;
        return Ok(Json(student));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;

    let student = StudentService::get_student_by_id(&state.db, id, school_id.into_inner()).await?;
    Ok(Json(student))
}

#[utoipa::path(
    put,
    path = "/api/students/{id}",
    params(
        ("id" = Uuid, Path, description = "Student ID")
    ),
    request_body = UpdateStudentDto,
    responses(
        (status = 200, description = "Student updated successfully", body = Student),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires students:update permission", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument(skip(state))]
pub async fn update_student(
    State(state): State<AppState>,
    RequireStudentsUpdate(auth_user): RequireStudentsUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateStudentDto>,
) -> Result<Json<Student>, AppError> {
    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation failed: {}", e)))?;

    // System admins can update any student
    if is_system_admin_jwt(&auth_user) {
        let student = StudentService::update_student_no_school_filter(
            &state.db,
            id,
            dto,
            state.cache.as_ref(),
        )
        .await?;
        return Ok(Json(student));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;

    let student = StudentService::update_student(
        &state.db,
        id,
        school_id.into_inner(),
        dto,
        state.cache.as_ref(),
    )
    .await?;
    Ok(Json(student))
}

#[utoipa::path(
    delete,
    path = "/api/students/{id}",
    params(
        ("id" = Uuid, Path, description = "Student ID")
    ),
    responses(
        (status = 200, description = "Student deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires students:delete permission", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument(skip(state))]
pub async fn delete_student(
    State(state): State<AppState>,
    RequireStudentsDelete(auth_user): RequireStudentsDelete,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // System admins can delete any student
    if is_system_admin_jwt(&auth_user) {
        StudentService::delete_student_no_school_filter(&state.db, id, state.cache.as_ref())
            .await?;
        return Ok(Json(json!({"message": "Student deleted successfully"})));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;

    StudentService::delete_student(&state.db, id, school_id.into_inner(), state.cache.as_ref())
        .await?;
    Ok(Json(json!({"message": "Student deleted successfully"})))
}
