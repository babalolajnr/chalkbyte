use axum::{
    Json, extract::Path, extract::Query, extract::State, extract::rejection::QueryRejection,
};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{
    CreateSchoolDto, PaginatedSchoolsResponse, PaginatedUsersResponse, School, SchoolFilterParams,
    SchoolFullInfo, UserFilterParams,
};
use crate::modules::users::service::UserService;
use crate::state::AppState;
use crate::utils::errors::AppError;

use super::service::SchoolService;

#[instrument(skip(db, auth_user), fields(user.id = %auth_user.0.sub, user.role = %auth_user.0.role))]
async fn get_admin_school_id(db: &sqlx::PgPool, auth_user: &AuthUser) -> Result<Uuid, AppError> {
    let user_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::bad_request(anyhow::anyhow!("Invalid user ID")))?;

    let user = UserService::get_user(db, user_id).await?;

    user.school_id.ok_or_else(|| {
        warn!(user.id = %user_id, "Admin user has no assigned school");
        AppError::forbidden("Admin must be assigned to a school".to_string())
    })
}

#[utoipa::path(
    post,
    path = "/api/schools",
    request_body = CreateSchoolDto,
    responses(
        (status = 201, description = "School created successfully", body = School),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user, dto), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    school.name = %dto.name
))]
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    if auth_user.0.role != "system_admin" {
        warn!(
            user.id = %auth_user.0.sub,
            user.role = %auth_user.0.role,
            "Unauthorized attempt to create school"
        );
        return Err(AppError::forbidden(
            "Only system admins can create schools".to_string(),
        ));
    }

    debug!(school.name = %dto.name, "Creating new school");

    let school = SchoolService::create_school(&state.db, dto).await?;

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
    params(
        ("name" = Option<String>, Query, description = "Filter by school name (partial match)"),
        ("address" = Option<String>, Query, description = "Filter by address (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of schools", body = PaginatedSchoolsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user, filters), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role
))]
pub async fn get_all_schools(
    State(state): State<AppState>,
    auth_user: AuthUser,
    filters: Result<Query<SchoolFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedSchoolsResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    if auth_user.0.role != "system_admin" {
        warn!(
            user.id = %auth_user.0.sub,
            user.role = %auth_user.0.role,
            "Unauthorized attempt to list all schools"
        );
        return Err(AppError::forbidden(
            "Only system admins can view all schools".to_string(),
        ));
    }

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
#[instrument(skip(state, _auth_user), fields(
    user.id = %_auth_user.0.sub,
    user.role = %_auth_user.0.role,
    school.id = %id
))]
pub async fn get_school(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<School>, AppError> {
    debug!("Fetching school by ID");

    let school = SchoolService::get_school_by_id(&state.db, id).await?;

    debug!(school.name = %school.name, "School found");

    Ok(Json(school))
}

#[utoipa::path(
    delete,
    path = "/api/schools/{id}",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 204, description = "School deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    school.id = %id
))]
pub async fn delete_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    if auth_user.0.role != "system_admin" {
        warn!(
            user.id = %auth_user.0.sub,
            school.id = %id,
            "Unauthorized attempt to delete school"
        );
        return Err(AppError::forbidden(
            "Only system admins can delete schools".to_string(),
        ));
    }

    debug!("Deleting school");

    SchoolService::delete_school(&state.db, id).await?;

    info!(school.id = %id, "School deleted successfully");

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/schools/{id}/students",
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("first_name" = Option<String>, Query, description = "Filter by first name (partial match)"),
        ("last_name" = Option<String>, Query, description = "Filter by last name (partial match)"),
        ("email" = Option<String>, Query, description = "Filter by email (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of students", body = PaginatedUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user, filters), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    school.id = %school_id
))]
pub async fn get_school_students(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(school_id): Path<Uuid>,
    filters: Result<Query<UserFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedUsersResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    if auth_user.0.role == "admin" {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.id = %auth_user.0.sub,
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access students from different school"
            );
            return Err(AppError::forbidden(
                "You can only view students from your own school".to_string(),
            ));
        }
    } else if auth_user.0.role != "system_admin" && auth_user.0.role != "teacher" {
        warn!(
            user.id = %auth_user.0.sub,
            user.role = %auth_user.0.role,
            "Unauthorized role attempted to view students"
        );
        return Err(AppError::forbidden(
            "Only admins and teachers can view students".to_string(),
        ));
    }

    debug!("Fetching students for school");

    let students = SchoolService::get_school_students(&state.db, school_id, filters).await?;

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
    params(
        ("id" = Uuid, Path, description = "School ID"),
        ("first_name" = Option<String>, Query, description = "Filter by first name (partial match)"),
        ("last_name" = Option<String>, Query, description = "Filter by last name (partial match)"),
        ("email" = Option<String>, Query, description = "Filter by email (partial match)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Paginated list of admins", body = PaginatedUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user, filters), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    school.id = %school_id
))]
pub async fn get_school_admins(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(school_id): Path<Uuid>,
    filters: Result<Query<UserFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedUsersResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;

    if auth_user.0.role != "system_admin" {
        warn!(
            user.id = %auth_user.0.sub,
            user.role = %auth_user.0.role,
            school.id = %school_id,
            "Unauthorized attempt to view school admins"
        );
        return Err(AppError::forbidden(
            "Only system admins can view school admins".to_string(),
        ));
    }

    debug!("Fetching admins for school");

    let admins = SchoolService::get_school_admins(&state.db, school_id, filters).await?;

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
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 200, description = "School full information with statistics", body = SchoolFullInfo),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, auth_user), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    school.id = %school_id
))]
pub async fn get_school_full_info(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(school_id): Path<Uuid>,
) -> Result<Json<SchoolFullInfo>, AppError> {
    if auth_user.0.role == "admin" {
        let admin_school_id = get_admin_school_id(&state.db, &auth_user).await?;
        if admin_school_id != school_id {
            warn!(
                user.id = %auth_user.0.sub,
                user.school_id = %admin_school_id,
                requested.school_id = %school_id,
                "Admin attempted to access info from different school"
            );
            return Err(AppError::forbidden(
                "You can only view information for your own school".to_string(),
            ));
        }
    } else if auth_user.0.role != "system_admin" {
        warn!(
            user.id = %auth_user.0.sub,
            user.role = %auth_user.0.role,
            "Unauthorized role attempted to view school full info"
        );
        return Err(AppError::forbidden(
            "Only system admins and school admins can view full school info".to_string(),
        ));
    }

    debug!("Fetching full school information");

    let school_info = SchoolService::get_school_full_info(&state.db, school_id).await?;

    debug!(
        school.name = %school_info.name,
        total_students = %school_info.total_students,
        total_teachers = %school_info.total_teachers,
        total_admins = %school_info.total_admins,
        "School full info fetched successfully"
    );

    Ok(Json(school_info))
}
