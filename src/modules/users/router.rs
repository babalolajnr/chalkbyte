use crate::modules::users::controller::{
    change_password, create_user, get_profile, get_users, update_profile,
};
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn init_users_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_users).post(create_user))
        .route("/profile", get(get_profile).put(update_profile))
        .route("/profile/change-password", post(change_password))
}
