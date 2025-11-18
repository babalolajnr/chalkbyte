use axum::{
    Router,
    routing::{get, post},
};

use crate::state::AppState;

use super::controller;

pub fn init_mfa_router() -> Router<AppState> {
    Router::new()
        .route("/status", get(controller::get_mfa_status))
        .route("/enable", post(controller::enable_mfa))
        .route("/verify", post(controller::verify_mfa))
        .route("/disable", post(controller::disable_mfa))
        .route(
            "/recovery-codes/regenerate",
            post(controller::regenerate_recovery_codes),
        )
}
