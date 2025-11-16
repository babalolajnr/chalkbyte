use sqlx::PgPool;

use crate::config::database::init_db_pool;
use crate::config::email::EmailConfig;
use crate::config::jwt::JwtConfig;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_config: JwtConfig,
    pub email_config: EmailConfig,
}

pub async fn init_app_state() -> AppState {
    AppState {
        db: init_db_pool().await,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
    }
}
