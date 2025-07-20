use crate::db::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::controller::register_user;

pub fn init_auth_router() -> Router<AppState> {
    Router::new().route("/register", post(register_user))
    // .route("/login", post(login_user))
    // .route("/profile", get(get_profile).put(update_profile))
}
