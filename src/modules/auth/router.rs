use crate::db::AppState;
use axum::{
    Router,
    routing::post,
};

use super::controller::login_user;

pub fn init_auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_user))
}
