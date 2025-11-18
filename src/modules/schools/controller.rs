use axum::{
    Json, extract::Path, extract::Query, extract::State, extract::rejection::QueryRejection,
};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{
    CreateSchoolDto, PaginatedSchoolsResponse, School, SchoolFilterParams,
};
use crate::state::AppState;
use crate::utils::errors::AppError;

use super::service::SchoolService;

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
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can create schools".to_string(),
        ));
    }

    let school = SchoolService::create_school(&state.db, dto).await?;
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
pub async fn get_all_schools(
    State(state): State<AppState>,
    auth_user: AuthUser,
    filters: Result<Query<SchoolFilterParams>, QueryRejection>,
) -> Result<Json<PaginatedSchoolsResponse>, AppError> {
    let Query(filters) = filters
        .map_err(|e| AppError::bad_request(anyhow::anyhow!("Invalid query parameters: {}", e)))?;
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can view all schools".to_string(),
        ));
    }

    let schools = SchoolService::get_all_schools(&state.db, filters).await?;
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
pub async fn get_school(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<School>, AppError> {
    let school = SchoolService::get_school_by_id(&state.db, id).await?;
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
pub async fn delete_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can delete schools".to_string(),
        ));
    }

    SchoolService::delete_school(&state.db, id).await?;
    Ok(())
}
