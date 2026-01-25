use axum::{
    extract::{Path, Query, State, rejection::QueryRejection},
    Json,
    body::Bytes,
    http::header,
};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use chalkbyte_core::AppError;
use chalkbyte_models::ids::{LevelId, SchoolId};

use crate::middleware::auth::{
    AuthUser, RequireSchoolsCreate, RequireSchoolsDelete, RequireSchoolsRead, RequireSchoolsUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::branches::model::{BranchFilterParams, PaginatedBranchesResponse};
use crate::modules::branches::service::BranchService;
use crate::modules::levels::model::{LevelFilterParams, PaginatedLevelsResponse};
use crate::modules::levels::service::LevelService;
use crate::modules::users::model::{
    CreateSchoolDto, PaginatedBasicUsersResponse, PaginatedSchoolsResponse, School,
    SchoolFilterParams, SchoolFullInfo, UserFilterParams,
};
use crate::state::AppState;
use crate::utils::auth_helpers::get_admin_school_id;

use super::model::FileMetadata;
use super::service::SchoolService;

#[utoipa::path(
    post,
    path = "/api/schools",
    summary = "Create school",
    request_body = CreateSchoolDto,
    responses(
        (status = 201, description = "School created successfully", body = School),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:create permission")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state), fields(school.name = %dto.name))]
pub async fn create_school(
    State(state): State<AppState>,
    RequireSchoolsCreate(_auth_user): RequireSchoolsCreate,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    debug!(school.name = %dto.name, "Creating new school");

    let school = SchoolService::create_school(&state.db, state.cache.as_ref(), dto).await?;

    info!(
        school.id = %school.id,
        school.name = %school.name,
        "School created successfully"
    );

    Ok(Json(school))
}

#[utoipa::path(
    get,
    path = "/api/schools",
    summary = "List schools",
    params(
        ("name" = Option<String>, Query, description = "Filter by school name (partial match)"),
        ("address" = Option<String>, Query, description = "Filter by address (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of schools", body = PaginatedSchoolsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, filters))]
pub async fn get_all_schools(
    State(state): State<AppState>,
    RequireSchoolsRead(_auth_user): RequireSchoolsRead,
    filters: Result<Query<SchoolFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedSchoolsResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    debug!(
        filter.name = ?filters.name,
        filter.address = ?filters.address,
        "Fetching schools with filters"
    );

    let schools = SchoolService::get_all_schools(&state.db, filters).await?;

    debug!(
        total = %schools.meta.total,
        returned = %schools.data.len(),
        "Schools fetched successfully"
    );

    Ok(Json(schools))
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}",
    summary = "Get school by ID",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 200, description = "School details", body = School),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state), fields(school.id = %id))]
pub async fn get_school(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<School>, AppError> {
    debug!("Fetching school by ID");

    let school = SchoolService::get_school_by_id(&state.db, state.cache.as_ref(), id).await?;

    debug!(school.name = %school.name, "School found");

    Ok(Json(school))
}

#[utoipa::path(
    delete,
    path = "/api/schools/{id}",
    summary = "Delete school",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 204, description = "School deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:delete permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state), fields(school.id = %id))]
pub async fn delete_school(
    State(state): State<AppState>,
    RequireSchoolsDelete(_auth_user): RequireSchoolsDelete,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    debug!("Deleting school");

    SchoolService::delete_school(&state.db, state.cache.as_ref(), id).await?;

    info!(school.id = %id, "School deleted successfully");

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/students",
    summary = "Get school students",
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("first_name" = Option<String>, Query, description = "Filter by first name (partial match)"),
        ("last_name" = Option<String>, Query, description = "Filter by last name (partial match)"),
        ("email" = Option<String>, Query, description = "Filter by email (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of students", body = PaginatedBasicUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, filters), fields(school.id = %school_id))]
pub async fn get_school_students(
    State(state): State<AppState>,
    RequireSchoolsRead(auth_user): RequireSchoolsRead,
    Path(school_id): Path<Uuid>,
    filters: Result<Query<UserFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedBasicUsersResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    let school_id = SchoolId::from(school_id);

    // School admins can only view students from their own school
    if !is_system_admin_jwt(&auth_user) {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access students from different school"
            );
            return Err(AppError::forbidden(
                "You can only view students from your own school".to_string(),
            ));
        }
    }

    debug!("Fetching students for school");

    let students =
        SchoolService::get_school_students(&state.db, school_id.into_inner(), filters).await?;

    debug!(
        total = %students.meta.total,
        returned = %students.data.len(),
        "Students fetched successfully"
    );

    Ok(Json(students))
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/admins",
    summary = "Get school admins",
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("first_name" = Option<String>, Query, description = "Filter by first name (partial match)"),
        ("last_name" = Option<String>, Query, description = "Filter by last name (partial match)"),
        ("email" = Option<String>, Query, description = "Filter by email (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of admins", body = PaginatedBasicUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission (system admins or school admin for own school)"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, filters), fields(school.id = %school_id))]
pub async fn get_school_admins(
    State(state): State<AppState>,
    RequireSchoolsRead(auth_user): RequireSchoolsRead,
    Path(school_id): Path<Uuid>,
    filters: Result<Query<UserFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedBasicUsersResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    let school_id = SchoolId::from(school_id);

    // System admins can view any school's admins
    // School admins can only view admins for their own school
    if !is_system_admin_jwt(&auth_user) {
        let user_school_id = auth_user
            .school_id()
            .ok_or_else(|| AppError::forbidden("User has no associated school".to_string()))?;
        if user_school_id != school_id {
            warn!("School admin attempted to view admins for different school");
            return Err(AppError::forbidden(
                "You can only view admins for your own school".to_string(),
            ));
        }
    }

    debug!("Fetching admins for school");

    let admins =
        SchoolService::get_school_admins(&state.db, school_id.into_inner(), filters).await?;

    debug!(
        total = %admins.meta.total,
        returned = %admins.data.len(),
        "School admins fetched successfully"
    );

    Ok(Json(admins))
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/full-info",
    summary = "Get school full info",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 200, description = "School full information with statistics", body = SchoolFullInfo),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state), fields(school.id = %school_id))]
pub async fn get_school_full_info(
    State(state): State<AppState>,
    RequireSchoolsRead(auth_user): RequireSchoolsRead,
    Path(school_id): Path<Uuid>,
) -> Result<Json<SchoolFullInfo>, AppError> {
    let school_id = SchoolId::from(school_id);

    // School admins can only view full info for their own school
    if !is_system_admin_jwt(&auth_user) {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access info from different school"
            );
            return Err(AppError::forbidden(
                "You can only view information for your own school".to_string(),
            ));
        }
    }

    debug!("Fetching full school information");

    let school_info =
        SchoolService::get_school_full_info(&state.db, school_id.into_inner()).await?;

    debug!(
        school.name = %school_info.name,
        total_students = %school_info.total_students,
        total_teachers = %school_info.total_teachers,
        total_admins = %school_info.total_admins,
        "School full info fetched successfully"
    );

    Ok(Json(school_info))
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/levels",
    summary = "Get school levels",
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("name" = Option<String>, Query, description = "Filter by level name (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of levels", body = PaginatedLevelsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, filters), fields(school.id = %school_id))]
pub async fn get_school_levels(
    State(state): State<AppState>,
    RequireSchoolsRead(auth_user): RequireSchoolsRead,
    Path(school_id): Path<Uuid>,
    filters: Result<Query<LevelFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedLevelsResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    let school_id = SchoolId::from(school_id);

    // School admins can only view levels from their own school
    if !is_system_admin_jwt(&auth_user) {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access levels from different school"
            );
            return Err(AppError::forbidden(
                "You can only view levels from your own school".to_string(),
            ));
        }
    }

    // Verify school exists
    SchoolService::get_school_by_id(&state.db, state.cache.as_ref(), school_id.into_inner())
        .await?;

    debug!("Fetching levels for school");

    let levels = LevelService::get_levels_by_school(&state.db, school_id, filters).await?;

    debug!(
        total = %levels.meta.total,
        returned = %levels.data.len(),
        "Levels fetched successfully"
    );

    Ok(Json(levels))
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/levels/{level_id}/branches",
    summary = "Get school level branches",
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("level_id" = Uuid, Path, description = "Level ID"),
        ("name" = Option<String>, Query, description = "Filter by branch name (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of branches", body = PaginatedBranchesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:read permission"),
        (status = 404, description = "School or level not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, filters), fields(school.id = %school_id, level.id = %level_id))]
pub async fn get_school_level_branches(
    State(state): State<AppState>,
    RequireSchoolsRead(auth_user): RequireSchoolsRead,
    Path((school_id, level_id)): Path<(Uuid, Uuid)>,
    filters: Result<Query<BranchFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedBranchesResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    let school_id = SchoolId::from(school_id);
    let level_id = LevelId::from(level_id);

    // School admins can only view branches from their own school
    if !is_system_admin_jwt(&auth_user) {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access branches from different school"
            );
            return Err(AppError::forbidden(
                "You can only view branches from your own school".to_string(),
            ));
        }
    }

    // Verify school exists
    SchoolService::get_school_by_id(&state.db, state.cache.as_ref(), school_id.into_inner())
        .await?;

    debug!("Fetching branches for level");

    let branches =
        BranchService::get_branches_by_level(&state.db, level_id, school_id, filters).await?;

    debug!(
        total = %branches.meta.total,
        returned = %branches.data.len(),
        "Branches fetched successfully"
    );

    Ok(Json(branches))
}

#[utoipa::path(
    post,
    path = "/api/schools/{id}/logo",
    summary = "Upload school logo",
    request_body(content = Vec<u8>, description = "Image file (PNG/JPEG/WebP, max 5MB)"),
    responses(
        (status = 200, description = "Logo uploaded successfully", body = School),
        (status = 400, description = "Invalid file format or size"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:update permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[axum::debug_handler]
#[instrument(skip(state, file_bytes), fields(school.id = %id))]
pub async fn upload_school_logo(
    State(state): State<AppState>,
    RequireSchoolsUpdate(auth_user): RequireSchoolsUpdate,
    Path(id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    file_bytes: Bytes,
) -> Result<Json<School>, AppError> {
    let school_id = SchoolId::from(id);

    // Authorization: system admin OR school admin for own school
    if !is_system_admin_jwt(&auth_user) {
        let user_school_id = auth_user
            .school_id()
            .ok_or_else(|| AppError::forbidden("User has no associated school".to_string()))?;
        if user_school_id != school_id {
            warn!(
                user.school_id = %user_school_id,
                requested.school_id = %school_id,
                "School admin attempted to upload logo for different school"
            );
            return Err(AppError::forbidden(
                "Can only update your own school's logo".to_string(),
            ));
        }
    }

    // Extract MIME type from Content-Type header
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let metadata = FileMetadata {
        mime_type: content_type.to_string(),
        size_bytes: file_bytes.len(),
        filename: "logo".to_string(),
    };

    debug!(
        school.id = %id,
        file.size = metadata.size_bytes,
        file.mime_type = %metadata.mime_type,
        "Uploading school logo"
    );

    let school = SchoolService::upload_school_logo(
        &state.db,
        state.cache.as_ref(),
        school_id,
        file_bytes.to_vec(),
        metadata,
        state.file_storage.as_ref(),
    )
    .await?;

    info!(school.id = %id, "School logo uploaded successfully");
    Ok(Json(school))
}

#[utoipa::path(
    delete,
    path = "/api/schools/{id}/logo",
    summary = "Delete school logo",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 204, description = "Logo deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires schools:update permission"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state), fields(school.id = %id))]
pub async fn delete_school_logo(
    State(state): State<AppState>,
    RequireSchoolsUpdate(auth_user): RequireSchoolsUpdate,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let school_id = SchoolId::from(id);

    // Authorization: system admin OR school admin for own school
    if !is_system_admin_jwt(&auth_user) {
        let user_school_id = auth_user
            .school_id()
            .ok_or_else(|| AppError::forbidden("User has no associated school".to_string()))?;
        if user_school_id != school_id {
            warn!(
                user.school_id = %user_school_id,
                requested.school_id = %school_id,
                "School admin attempted to delete logo for different school"
            );
            return Err(AppError::forbidden(
                "Can only delete your own school's logo".to_string(),
            ));
        }
    }

    debug!(school.id = %id, "Deleting school logo");

    SchoolService::delete_school_logo(
        &state.db,
        state.cache.as_ref(),
        school_id,
        state.file_storage.as_ref(),
    )
    .await?;

    info!(school.id = %id, "School logo deleted successfully");
    Ok(())
}
