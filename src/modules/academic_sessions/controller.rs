use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

use chalkbyte_core::AppError;
use chalkbyte_models::ids::AcademicSessionId;

use crate::middleware::auth::{
    RequireAcademicSessionsCreate, RequireAcademicSessionsDelete, RequireAcademicSessionsRead,
    RequireAcademicSessionsUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::academic_sessions::model::{
    AcademicSession, AcademicSessionFilterParams, AcademicSessionWithStats,
    CreateAcademicSessionDto, PaginatedAcademicSessionsResponse, UpdateAcademicSessionDto,
};
use crate::modules::academic_sessions::service::AcademicSessionService;
use crate::state::AppState;
use crate::utils::auth_helpers::{get_admin_school_id, get_school_id_for_scoped_operation};

/// Create a new academic session
#[utoipa::path(
    post,
    path = "/api/academic-sessions",
    request_body = CreateAcademicSessionDto,
    responses(
        (status = 201, description = "Academic session created successfully", body = AcademicSession),
        (status = 400, description = "Invalid input or missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:create permission")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsCreate(auth_user): RequireAcademicSessionsCreate,
    Json(dto): Json<CreateAcademicSessionDto>,
) -> Result<(StatusCode, Json<AcademicSession>), AppError> {
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, dto.school_id).await?;

    dto.validate()?;

    let session =
        AcademicSessionService::create_academic_session(&state.db, school_id, dto).await?;

    Ok((StatusCode::CREATED, Json(session)))
}

/// Get all academic sessions for a school
#[utoipa::path(
    get,
    path = "/api/academic-sessions",
    params(AcademicSessionFilterParams),
    responses(
        (status = 200, description = "List of academic sessions", body = PaginatedAcademicSessionsResponse),
        (status = 400, description = "Missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:read permission")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_academic_sessions(
    State(state): State<AppState>,
    RequireAcademicSessionsRead(auth_user): RequireAcademicSessionsRead,
    Query(filters): Query<AcademicSessionFilterParams>,
) -> Result<Json<PaginatedAcademicSessionsResponse>, AppError> {
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, filters.school_id).await?;

    let sessions =
        AcademicSessionService::get_academic_sessions_by_school(&state.db, school_id, filters)
            .await?;

    Ok(Json(sessions))
}

/// Get the active academic session for a school
#[utoipa::path(
    get,
    path = "/api/academic-sessions/active",
    params(
        ("school_id" = Option<Uuid>, Query, description = "School ID (required for system admins)")
    ),
    responses(
        (status = 200, description = "Active academic session", body = Option<AcademicSessionWithStats>),
        (status = 400, description = "Missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:read permission")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_active_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsRead(auth_user): RequireAcademicSessionsRead,
    Query(params): Query<AcademicSessionFilterParams>,
) -> Result<Json<Option<AcademicSessionWithStats>>, AppError> {
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, params.school_id).await?;

    let session = AcademicSessionService::get_active_academic_session(&state.db, school_id).await?;

    Ok(Json(session))
}

/// Get an academic session by ID
#[utoipa::path(
    get,
    path = "/api/academic-sessions/{id}",
    params(
        ("id" = Uuid, Path, description = "Academic session ID")
    ),
    responses(
        (status = 200, description = "Academic session details", body = AcademicSessionWithStats),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:read permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_academic_session_by_id(
    State(state): State<AppState>,
    RequireAcademicSessionsRead(auth_user): RequireAcademicSessionsRead,
    Path(id): Path<Uuid>,
) -> Result<Json<AcademicSessionWithStats>, AppError> {
    let session_id = AcademicSessionId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let session = AcademicSessionService::get_academic_session_by_id_no_school_filter(
            &state.db, session_id,
        )
        .await?;
        return Ok(Json(session));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let session =
        AcademicSessionService::get_academic_session_by_id(&state.db, session_id, school_id)
            .await?;

    Ok(Json(session))
}

/// Update an academic session
#[utoipa::path(
    put,
    path = "/api/academic-sessions/{id}",
    params(
        ("id" = Uuid, Path, description = "Academic session ID")
    ),
    request_body = UpdateAcademicSessionDto,
    responses(
        (status = 200, description = "Academic session updated successfully", body = AcademicSession),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:update permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsUpdate(auth_user): RequireAcademicSessionsUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateAcademicSessionDto>,
) -> Result<Json<AcademicSession>, AppError> {
    dto.validate()?;
    let session_id = AcademicSessionId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let session = AcademicSessionService::update_academic_session_no_school_filter(
            &state.db, session_id, dto,
        )
        .await?;
        return Ok(Json(session));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let session =
        AcademicSessionService::update_academic_session(&state.db, session_id, school_id, dto)
            .await?;

    Ok(Json(session))
}

/// Delete an academic session
#[utoipa::path(
    delete,
    path = "/api/academic-sessions/{id}",
    params(
        ("id" = Uuid, Path, description = "Academic session ID")
    ),
    responses(
        (status = 204, description = "Academic session deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:delete permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsDelete(auth_user): RequireAcademicSessionsDelete,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session_id = AcademicSessionId::from(id);

    if is_system_admin_jwt(&auth_user) {
        AcademicSessionService::delete_academic_session_no_school_filter(&state.db, session_id)
            .await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    AcademicSessionService::delete_academic_session(&state.db, session_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Activate an academic session
#[utoipa::path(
    post,
    path = "/api/academic-sessions/{id}/activate",
    params(
        ("id" = Uuid, Path, description = "Academic session ID")
    ),
    responses(
        (status = 200, description = "Academic session activated successfully", body = AcademicSession),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:update permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn activate_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsUpdate(auth_user): RequireAcademicSessionsUpdate,
    Path(id): Path<Uuid>,
) -> Result<Json<AcademicSession>, AppError> {
    let session_id = AcademicSessionId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let session = AcademicSessionService::activate_academic_session_no_school_filter(
            &state.db, session_id,
        )
        .await?;
        return Ok(Json(session));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let session =
        AcademicSessionService::activate_academic_session(&state.db, session_id, school_id).await?;

    Ok(Json(session))
}

/// Deactivate an academic session
#[utoipa::path(
    post,
    path = "/api/academic-sessions/{id}/deactivate",
    params(
        ("id" = Uuid, Path, description = "Academic session ID")
    ),
    responses(
        (status = 200, description = "Academic session deactivated successfully", body = AcademicSession),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires academic_sessions:update permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Academic Sessions",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn deactivate_academic_session(
    State(state): State<AppState>,
    RequireAcademicSessionsUpdate(auth_user): RequireAcademicSessionsUpdate,
    Path(id): Path<Uuid>,
) -> Result<Json<AcademicSession>, AppError> {
    let session_id = AcademicSessionId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let session = AcademicSessionService::deactivate_academic_session_no_school_filter(
            &state.db, session_id,
        )
        .await?;
        return Ok(Json(session));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let session =
        AcademicSessionService::deactivate_academic_session(&state.db, session_id, school_id)
            .await?;

    Ok(Json(session))
}
