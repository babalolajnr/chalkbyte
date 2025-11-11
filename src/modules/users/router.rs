use crate::db::AppState;
use crate::modules::users::controller::{create_user, get_profile, get_users};
use axum::{Router, routing::get};

pub fn init_users_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_users).post(create_user))
        .route("/profile", get(get_profile))
}
