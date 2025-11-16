use crate::db::AppState;
use axum::{Router, routing::post};

use super::controller::{forgot_password, login_user, reset_password};

pub fn init_auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_user))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
}
