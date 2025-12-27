use crate::middleware::auth::AuthUser;
use crate::middleware::role::{get_user_id_from_auth, is_admin};
use crate::modules::auth::controller::ErrorResponse;
use crate::modules::students::model::{
    CreateStudentDto, PaginatedStudentsResponse, PaginationMeta, QueryParams, Student,
    UpdateStudentDto,
};
use crate::modules::students::service::StudentService;
use crate::modules::users::service::UserService;
use crate::state::AppState;
use crate::utils::errors::AppError;
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
        (status = 403, description = "Forbidden - School Admin only", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument]
pub async fn create_student(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateStudentDto>,
) -> Result<Json<Student>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;

    if !is_admin(&state.db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can create students".to_string(),
        ));
    }

    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation failed: {}", e)))?;

    let requester = UserService::get_user(&state.db, user_id).await?;
    let school_id = requester
        .school_id
        .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

    let student = StudentService::create_student(&state.db, dto, school_id).await?;
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
        (status = 403, description = "Forbidden - School Admin only", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument]
pub async fn get_students(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<QueryParams>,
) -> Result<Json<PaginatedStudentsResponse>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;

    if !is_admin(&state.db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can list students".to_string(),
        ));
    }

    let requester = UserService::get_user(&state.db, user_id).await?;
    let school_id = requester
        .school_id
        .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

    let limit = params.limit();
    let offset = params.offset();
    let page = params.page();

    let (students, total) =
        StudentService::get_students_by_school(&state.db, school_id, limit, offset).await?;

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
        (status = 403, description = "Forbidden - School Admin only", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument]
pub async fn get_student(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Student>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;

    if !is_admin(&state.db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can view students".to_string(),
        ));
    }

    let requester = UserService::get_user(&state.db, user_id).await?;
    let school_id = requester
        .school_id
        .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

    let student = StudentService::get_student_by_id(&state.db, id, school_id).await?;
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
        (status = 403, description = "Forbidden - School Admin only", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument]
pub async fn update_student(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateStudentDto>,
) -> Result<Json<Student>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;

    if !is_admin(&state.db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can update students".to_string(),
        ));
    }

    dto.validate()
        .map_err(|e| AppError::unprocessable(anyhow::anyhow!("Validation failed: {}", e)))?;

    let requester = UserService::get_user(&state.db, user_id).await?;
    let school_id = requester
        .school_id
        .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

    let student = StudentService::update_student(&state.db, id, school_id, dto).await?;
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
        (status = 403, description = "Forbidden - School Admin only", body = ErrorResponse),
        (status = 404, description = "Student not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Students"
)]
#[instrument]
pub async fn delete_student(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = get_user_id_from_auth(&auth_user)?;

    if !is_admin(&state.db, user_id).await? {
        return Err(AppError::forbidden(
            "Only school admins can delete students".to_string(),
        ));
    }

    let requester = UserService::get_user(&state.db, user_id).await?;
    let school_id = requester
        .school_id
        .ok_or_else(|| AppError::forbidden("Admin must be assigned to a school".to_string()))?;

    StudentService::delete_student(&state.db, id, school_id).await?;
    Ok(Json(json!({"message": "Student deleted successfully"})))
}
