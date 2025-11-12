use axum::{extract::State, Json, extract::Path};
use uuid::Uuid;

use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{CreateSchoolDto, School};
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
        return Err(AppError::forbidden("Only system admins can create schools".to_string()));
    }

    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}

#[utoipa::path(
    get,
    path = "/api/schools",
    responses(
        (status = 200, description = "List of all schools", body = Vec<School>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn get_all_schools(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<School>>, AppError> {
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden("Only system admins can view all schools".to_string()));
    }

    let schools = SchoolService::get_all_schools(&state.db).await?;
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
        return Err(AppError::forbidden("Only system admins can delete schools".to_string()));
    }

    SchoolService::delete_school(&state.db, id).await?;
    Ok(())
}
