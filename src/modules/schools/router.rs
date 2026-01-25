use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

use super::controller::{
    create_school, delete_school, delete_school_logo, get_all_schools, get_school,
    get_school_admins, get_school_full_info, get_school_level_branches, get_school_levels,
    get_school_students, upload_school_logo,
};

pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_school).get(get_all_schools))
        .route("/{id}", get(get_school).delete(delete_school))
        .route(
            "/{id}/logo",
            post(upload_school_logo).delete(delete_school_logo),
        )
        .route("/{id}/students", get(get_school_students))
        .route("/{id}/admins", get(get_school_admins))
        .route("/{id}/full-info", get(get_school_full_info))
        .route("/{id}/levels", get(get_school_levels))
        .route(
            "/{id}/levels/{level_id}/branches",
            get(get_school_level_branches),
        )
}
