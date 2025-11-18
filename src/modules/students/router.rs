use crate::modules::students::controller::{
    create_student, delete_student, get_student, get_students, update_student,
};
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn init_students_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_student).get(get_students))
        .route(
            "/{id}",
            get(get_student).put(update_student).delete(delete_student),
        )
}
