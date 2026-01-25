use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;
use validator::Validate;

use chalkbyte_core::AppError;
use chalkbyte_models::ids::{AcademicSessionId, TermId};

use crate::middleware::auth::{
    RequireTermsCreate, RequireTermsDelete, RequireTermsRead, RequireTermsUpdate,
};
use crate::middleware::role::is_system_admin_jwt;
use crate::modules::terms::model::{
    CreateTermDto, PaginatedTermsResponse, Term, TermFilterParams, TermWithSessionInfo,
    UpdateTermDto,
};
use crate::modules::terms::service::TermService;
use crate::state::AppState;
use crate::utils::auth_helpers::{get_admin_school_id, get_school_id_for_scoped_operation};

/// Create a new term within an academic session
#[utoipa::path(
    post,
    path = "/api/academic-sessions/{session_id}/terms",
    summary = "Create term",
    params(
        ("session_id" = Uuid, Path, description = "Academic session ID")
    ),
    request_body = CreateTermDto,
    responses(
        (status = 201, description = "Term created successfully", body = Term),
        (status = 400, description = "Invalid input or date validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:create permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_session_term(
    State(state): State<AppState>,
    RequireTermsCreate(_auth_user): RequireTermsCreate,
    Path(session_id): Path<Uuid>,
    Json(dto): Json<CreateTermDto>,
) -> Result<(StatusCode, Json<Term>), AppError> {
    dto.validate()?;

    let session_id = AcademicSessionId::from(session_id);
    let term = TermService::create_term(&state.db, session_id, dto).await?;

    Ok((StatusCode::CREATED, Json(term)))
}

/// Get all terms for an academic session
#[utoipa::path(
    get,
    path = "/api/academic-sessions/{session_id}/terms",
    summary = "List session terms",
    params(
        ("session_id" = Uuid, Path, description = "Academic session ID"),
        TermFilterParams
    ),
    responses(
        (status = 200, description = "List of terms", body = PaginatedTermsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:read permission"),
        (status = 404, description = "Academic session not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_session_terms(
    State(state): State<AppState>,
    RequireTermsRead(_auth_user): RequireTermsRead,
    Path(session_id): Path<Uuid>,
    Query(filters): Query<TermFilterParams>,
) -> Result<Json<PaginatedTermsResponse>, AppError> {
    let session_id = AcademicSessionId::from(session_id);
    let terms = TermService::get_terms_by_session(&state.db, session_id, filters).await?;

    Ok(Json(terms))
}

/// Get the current term for a school's active session
#[utoipa::path(
    get,
    path = "/api/terms/current",
    summary = "Get current term",
    params(
        ("school_id" = Option<Uuid>, Query, description = "School ID (required for system admins)")
    ),
    responses(
        (status = 200, description = "Current term", body = Option<TermWithSessionInfo>),
        (status = 400, description = "Missing school_id for system admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:read permission")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_current_term(
    State(state): State<AppState>,
    RequireTermsRead(auth_user): RequireTermsRead,
    Query(params): Query<TermFilterParams>,
) -> Result<Json<Option<TermWithSessionInfo>>, AppError> {
    let school_id =
        get_school_id_for_scoped_operation(&state.db, &auth_user, params.school_id).await?;

    let term = TermService::get_current_term(&state.db, school_id).await?;

    Ok(Json(term))
}

/// Get a term by ID
#[utoipa::path(
    get,
    path = "/api/terms/{id}",
    summary = "Get term by ID",
    params(
        ("id" = Uuid, Path, description = "Term ID")
    ),
    responses(
        (status = 200, description = "Term details", body = TermWithSessionInfo),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:read permission"),
        (status = 404, description = "Term not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_term_by_id(
    State(state): State<AppState>,
    RequireTermsRead(auth_user): RequireTermsRead,
    Path(id): Path<Uuid>,
) -> Result<Json<TermWithSessionInfo>, AppError> {
    let term_id = TermId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let term = TermService::get_term_by_id(&state.db, term_id).await?;
        return Ok(Json(term));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let term =
        TermService::get_term_by_id_with_school_filter(&state.db, term_id, school_id).await?;

    Ok(Json(term))
}

/// Update a term
#[utoipa::path(
    put,
    path = "/api/terms/{id}",
    summary = "Update term",
    params(
        ("id" = Uuid, Path, description = "Term ID")
    ),
    request_body = UpdateTermDto,
    responses(
        (status = 200, description = "Term updated successfully", body = Term),
        (status = 400, description = "Invalid input or date validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:update permission"),
        (status = 404, description = "Term not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_term(
    State(state): State<AppState>,
    RequireTermsUpdate(auth_user): RequireTermsUpdate,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateTermDto>,
) -> Result<Json<Term>, AppError> {
    dto.validate()?;
    let term_id = TermId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let term = TermService::update_term(&state.db, term_id, dto).await?;
        return Ok(Json(term));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let term =
        TermService::update_term_with_school_filter(&state.db, term_id, school_id, dto).await?;

    Ok(Json(term))
}

/// Delete a term
#[utoipa::path(
    delete,
    path = "/api/terms/{id}",
    summary = "Delete term",
    params(
        ("id" = Uuid, Path, description = "Term ID")
    ),
    responses(
        (status = 204, description = "Term deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:delete permission"),
        (status = 404, description = "Term not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn delete_term(
    State(state): State<AppState>,
    RequireTermsDelete(auth_user): RequireTermsDelete,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let term_id = TermId::from(id);

    if is_system_admin_jwt(&auth_user) {
        TermService::delete_term(&state.db, term_id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    TermService::delete_term_with_school_filter(&state.db, term_id, school_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Set a term as the current term
#[utoipa::path(
    post,
    path = "/api/terms/{id}/set-current",
    summary = "Set current term",
    params(
        ("id" = Uuid, Path, description = "Term ID")
    ),
    responses(
        (status = 200, description = "Term set as current successfully", body = Term),
        (status = 400, description = "Session is not active"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires terms:update permission"),
        (status = 404, description = "Term not found")
    ),
    tag = "Terms",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn set_current_term(
    State(state): State<AppState>,
    RequireTermsUpdate(auth_user): RequireTermsUpdate,
    Path(id): Path<Uuid>,
) -> Result<Json<Term>, AppError> {
    let term_id = TermId::from(id);

    if is_system_admin_jwt(&auth_user) {
        let term = TermService::set_current_term(&state.db, term_id).await?;
        return Ok(Json(term));
    }

    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    let term =
        TermService::set_current_term_with_school_filter(&state.db, term_id, school_id).await?;

    Ok(Json(term))
}
