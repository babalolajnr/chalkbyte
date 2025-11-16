use crate::db::AppState;
use axum::{Router, routing::post};

use super::controller::{
    forgot_password, login_user, reset_password, verify_mfa_login, verify_mfa_recovery_login,
};

pub fn init_auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_user))
        .route("/mfa/verify", post(verify_mfa_login))
        .route("/mfa/recovery", post(verify_mfa_recovery_login))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
}
