use crate::db::AppState;
use crate::modules::users::router::init_router as init_users_router;
use axum::Router;

pub fn init_router(state: AppState) -> Router {
    Router::new()
        .nest("/api", init_users_router())
        .with_state(state)
}
