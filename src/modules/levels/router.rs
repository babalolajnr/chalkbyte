use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::state::AppState;

use super::controller::{
    assign_students_to_level, create_level, delete_level, get_level_by_id, get_levels,
    get_students_in_level, move_student_to_level, remove_student_from_level, update_level,
};

pub fn init_levels_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_level).get(get_levels))
        .route(
            "/{id}",
            get(get_level_by_id).put(update_level).delete(delete_level),
        )
        .route(
            "/{id}/students",
            post(assign_students_to_level).get(get_students_in_level),
        )
        .route("/students/{student_id}/move", patch(move_student_to_level))
        .route("/students/{student_id}", delete(remove_student_from_level))
}
