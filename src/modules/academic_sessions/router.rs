use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

use super::controller::{
    activate_academic_session, create_academic_session, deactivate_academic_session,
    delete_academic_session, get_academic_session_by_id, get_academic_sessions,
    get_active_academic_session, update_academic_session,
};

pub fn init_academic_sessions_router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(create_academic_session).get(get_academic_sessions),
        )
        .route("/active", get(get_active_academic_session))
        .route(
            "/{id}",
            get(get_academic_session_by_id)
                .put(update_academic_session)
                .delete(delete_academic_session),
        )
        .route("/{id}/activate", post(activate_academic_session))
        .route("/{id}/deactivate", post(deactivate_academic_session))
}
