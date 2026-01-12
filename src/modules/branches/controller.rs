use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

use chalkbyte_core::AppError;
use chalkbyte_models::ids::{BranchId, LevelId, UserId};

use crate::middleware::auth::{
    RequireBranchesAssignStudents, RequireBranchesCreate, RequireBranchesDelete,
    RequireBranchesRead, RequireBranchesUpdate,
};
use crate::middleware::role::{get_admin_school_id, is_system_admin_jwt};
use crate::modules::branches::model::{
    AssignStudentsToBranchDto, Branch, BranchFilterParams, BranchWithStats, BulkAssignResponse,
    CreateBranchDto, MoveStudentToBranchDto, PaginatedBranchesResponse, UpdateBranchDto,
};
use crate::modules::branches::service::BranchService;
use crate::modules::users::model::User;
use crate::state::AppState;

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
        (status = 403, description = "Forbidden - requires branches:create permission")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_branch(
    State(state): State<AppState>,
    RequireBranchesCreate(auth_user): RequireBranchesCreate,
    Path(level_id): Path<Uuid>,
    Json(dto): Json<CreateBranchDto>,
) -> Result<(StatusCode, Json<Branch>), AppError> {
    dto.validate()?;

    let level_id = LevelId::from(level_id);

    // System admins can create branches for any level
    if is_system_admin_jwt(&auth_user) {
        let branch = BranchService::create_branch_no_school_filter(
            &state.db,
            state.cache.as_ref(),
            level_id,
            dto,
        )
        .await?;
        return Ok((StatusCode::CREATED, Json(branch)));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let branch =
        BranchService::create_branch(&state.db, state.cache.as_ref(), level_id, school_id, dto)
            .await?;

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
        (status = 403, description = "Forbidden - requires branches:read permission")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_branches(
    State(state): State<AppState>,
    RequireBranchesRead(auth_user): RequireBranchesRead,
    Path(level_id): Path<Uuid>,
    Query(filters): Query<BranchFilterParams>,
) -> Result<Json<PaginatedBranchesResponse>, AppError> {
    let level_id = LevelId::from(level_id);

    // System admins can view branches for any level
    if is_system_admin_jwt(&auth_user) {
        let branches =
            BranchService::get_branches_by_level_no_school_filter(&state.db, level_id, filters)
                .await?;
        return Ok(Json(branches));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires branches:read permission"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_branch_by_id(
    State(state): State<AppState>,
    RequireBranchesRead(auth_user): RequireBranchesRead,
    Path(id): Path<Uuid>,
) -> Result<Json<BranchWithStats>, AppError> {
    let id = BranchId::from(id);

    // System admins can view any branch
    if is_system_admin_jwt(&auth_user) {
        let branch = BranchService::get_branch_by_id_no_school_filter(&state.db, id).await?;
        return Ok(Json(branch));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires branches:update permission"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_branch(
    State(state): State<AppState>,
    RequireBranchesUpdate(auth_user): RequireBranchesUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateBranchDto>,
) -> Result<Json<Branch>, AppError> {
    dto.validate()?;

    let id = BranchId::from(id);

    // System admins can update any branch
    if is_system_admin_jwt(&auth_user) {
        let branch =
            BranchService::update_branch_no_school_filter(&state.db, state.cache.as_ref(), id, dto)
                .await?;
        return Ok(Json(branch));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let branch =
        BranchService::update_branch(&state.db, state.cache.as_ref(), id, school_id, dto).await?;

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
        (status = 403, description = "Forbidden - requires branches:delete permission"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_branch(
    State(state): State<AppState>,
    RequireBranchesDelete(auth_user): RequireBranchesDelete,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let id = BranchId::from(id);

    // System admins can delete any branch
    if is_system_admin_jwt(&auth_user) {
        BranchService::delete_branch_no_school_filter(&state.db, state.cache.as_ref(), id, None)
            .await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    BranchService::delete_branch(&state.db, state.cache.as_ref(), id, school_id, None).await?;

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
        (status = 403, description = "Forbidden - requires branches:assign_students permission"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn assign_students_to_branch(
    State(state): State<AppState>,
    RequireBranchesAssignStudents(auth_user): RequireBranchesAssignStudents,
    Path(id): Path<Uuid>,
    Json(dto): Json<AssignStudentsToBranchDto>,
) -> Result<Json<BulkAssignResponse>, AppError> {
    dto.validate()?;

    let id = BranchId::from(id);

    // System admins can assign students to any branch
    if is_system_admin_jwt(&auth_user) {
        let response =
            BranchService::assign_students_to_branch_no_school_filter(&state.db, id, dto).await?;
        return Ok(Json(response));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires branches:read permission"),
        (status = 404, description = "Branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_students_in_branch(
    State(state): State<AppState>,
    RequireBranchesRead(auth_user): RequireBranchesRead,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<User>>, AppError> {
    let id = BranchId::from(id);

    // System admins can view students in any branch
    if is_system_admin_jwt(&auth_user) {
        let students =
            BranchService::get_students_in_branch_no_school_filter(&state.db, id).await?;
        return Ok(Json(students));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires branches:assign_students permission"),
        (status = 404, description = "Student or branch not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn move_student_to_branch(
    State(state): State<AppState>,
    RequireBranchesAssignStudents(auth_user): RequireBranchesAssignStudents,
    Path(student_id): Path<Uuid>,
    Json(dto): Json<MoveStudentToBranchDto>,
) -> Result<StatusCode, AppError> {
    dto.validate()?;

    let student_id = UserId::from(student_id);

    // System admins can move any student
    if is_system_admin_jwt(&auth_user) {
        BranchService::move_student_to_branch_no_school_filter(&state.db, student_id, dto).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
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
        (status = 403, description = "Forbidden - requires branches:assign_students permission"),
        (status = 404, description = "Student not found")
    ),
    tag = "Branches",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn remove_student_from_branch(
    State(state): State<AppState>,
    RequireBranchesAssignStudents(auth_user): RequireBranchesAssignStudents,
    Path(student_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let student_id = UserId::from(student_id);

    // System admins can remove any student from branch
    if is_system_admin_jwt(&auth_user) {
        BranchService::remove_student_from_branch_no_school_filter(&state.db, student_id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    BranchService::remove_student_from_branch(&state.db, student_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
