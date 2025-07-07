use crate::db::AppState;
use crate::modules::users::controller::{create_user, get_users};
use axum::{Router, routing::get};

pub fn init_router() -> Router<AppState> {
    Router::new().route("/users", get(get_users).post(create_user))
}
