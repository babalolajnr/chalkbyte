use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

use super::controller::{
    create_session_term, delete_term, get_current_term, get_session_terms, get_term_by_id,
    set_current_term, update_term,
};

/// Initialize the terms router for nested routes under academic sessions
/// Routes: POST /, GET /
pub fn init_session_terms_router() -> Router<AppState> {
    Router::new().route("/", post(create_session_term).get(get_session_terms))
}

/// Initialize the standalone terms router
/// Routes: GET /current, GET /{id}, PUT /{id}, DELETE /{id}, POST /{id}/set-current
pub fn init_terms_router() -> Router<AppState> {
    Router::new()
        .route("/current", get(get_current_term))
        .route(
            "/{id}",
            get(get_term_by_id).put(update_term).delete(delete_term),
        )
        .route("/{id}/set-current", post(set_current_term))
}
