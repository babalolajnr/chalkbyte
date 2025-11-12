use axum::{
    Router,
    routing::{get, post},
};

use crate::db::AppState;

use super::controller::{create_school, delete_school, get_all_schools, get_school};

pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_school).get(get_all_schools))
        .route("/{id}", get(get_school).delete(delete_school))
}
