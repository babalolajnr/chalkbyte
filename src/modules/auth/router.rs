use crate::db::AppState;
use axum::{
    Router,
    routing::post,
};

use super::controller::{login_user, register_user};

pub fn init_auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login_user))
}
