use sqlx::PgPool;

use crate::config::database::init_db_pool;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: PgPool,
}

pub async fn init_app_state() -> AppState {
    AppState {
        db: init_db_pool().await,
    }
}
