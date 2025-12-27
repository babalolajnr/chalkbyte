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
use crate::modules::branches::model::{
    AssignStudentsToBranchDto, Branch, BranchFilterParams, BranchWithStats, BulkAssignResponse,
    CreateBranchDto, MoveStudentToBranchDto, PaginatedBranchesResponse, UpdateBranchDto,
};
use crate::modules::branches::service::BranchService;
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
    path = "/api/levels/{level_id}/branches",
    params(
        ("level_id" = Uuid, Path, description = "Level ID")
    ),
    request_body = CreateBranchDto,
    responses(
        (status = 201, description = "Branch created successfully", body = Branch),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(level_id): Path<Uuid>,
    Json(dto): Json<CreateBranchDto>,
) -> Result<(StatusCode, Json<Branch>), AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let branch = BranchService::create_branch(&state.db, level_id, school_id, dto).await?;

    Ok((StatusCode::CREATED, Json(branch)))
}

#[utoipa::path(
    get,
    path = "/api/levels/{level_id}/branches",
    params(
        ("level_id" = Uuid, Path, description = "Level ID"),
        BranchFilterParams
    ),
    responses(
        (status = 200, description = "List of branches", body = PaginatedBranchesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_branches(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(level_id): Path<Uuid>,
    Query(filters): Query<BranchFilterParams>,
) -> Result<Json<PaginatedBranchesResponse>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let branches =
        BranchService::get_branches_by_level(&state.db, level_id, school_id, filters).await?;

    Ok(Json(branches))
}

#[utoipa::path(
    get,
    path = "/api/branches/{id}",
    params(
        ("id" = Uuid, Path, description = "Branch ID")
    ),
    responses(
        (status = 200, description = "Branch details", body = BranchWithStats),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_branch_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BranchWithStats>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let branch = BranchService::get_branch_by_id(&state.db, id, school_id).await?;

    Ok(Json(branch))
}

#[utoipa::path(
    put,
    path = "/api/branches/{id}",
    params(
        ("id" = Uuid, Path, description = "Branch ID")
    ),
    request_body = UpdateBranchDto,
    responses(
        (status = 200, description = "Branch updated successfully", body = Branch),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateBranchDto>,
) -> Result<Json<Branch>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let branch = BranchService::update_branch(&state.db, id, school_id, dto).await?;

    Ok(Json(branch))
}

#[utoipa::path(
    delete,
    path = "/api/branches/{id}",
    params(
        ("id" = Uuid, Path, description = "Branch ID")
    ),
    responses(
        (status = 204, description = "Branch deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    BranchService::delete_branch(&state.db, id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/branches/{id}/students",
    params(
        ("id" = Uuid, Path, description = "Branch ID")
    ),
    request_body = AssignStudentsToBranchDto,
    responses(
        (status = 200, description = "Students assigned to branch", body = BulkAssignResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn assign_students_to_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignStudentsToBranchDto>,
) -> Result<Json<BulkAssignResponse>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    let response = BranchService::assign_students_to_branch(&state.db, id, school_id, dto).await?;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/branches/{id}/students",
    params(
        ("id" = Uuid, Path, description = "Branch ID")
    ),
    responses(
        (status = 200, description = "List of students in branch", body = Vec<User>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_students_in_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<User>>, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    let students = BranchService::get_students_in_branch(&state.db, id, school_id).await?;

    Ok(Json(students))
}

#[utoipa::path(
    patch,
    path = "/api/branches/students/{student_id}/move",
    params(
        ("student_id" = Uuid, Path, description = "Student ID")
    ),
    request_body = MoveStudentToBranchDto,
    responses(
        (status = 204, description = "Student moved successfully"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Student or branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn move_student_to_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
    Json(dto): Json<MoveStudentToBranchDto>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    dto.validate()?;

    BranchService::move_student_to_branch(&state.db, student_id, school_id, dto).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/branches/students/{student_id}",
    params(
        ("student_id" = Uuid, Path, description = "Student ID")
    ),
    responses(
        (status = 204, description = "Student removed from branch"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Student not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn remove_student_from_branch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;

    BranchService::remove_student_from_branch(&state.db, student_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
