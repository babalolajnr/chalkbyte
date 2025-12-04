use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::state::AppState;

use super::controller::{
    assign_students_to_branch, create_branch, delete_branch, get_branch_by_id, get_branches,
    get_students_in_branch, move_student_to_branch, remove_student_from_branch, update_branch,
};

pub fn init_branches_router() -> Router<AppState> {
    Router::new()
        .route("/students/move/{student_id}", patch(move_student_to_branch))
        .route("/students/{student_id}", delete(remove_student_from_branch))
        .route(
            "/{id}/students",
            post(assign_students_to_branch).get(get_students_in_branch),
        )
        .route(
            "/{id}",
            get(get_branch_by_id)
                .put(update_branch)
                .delete(delete_branch),
        )
}

pub fn init_level_branches_router() -> Router<AppState> {
    Router::new().route("/", post(create_branch).get(get_branches))
}
