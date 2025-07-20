use sqlx::PgPool;
use tracing::instrument;

use crate::utils::errors::AppError;

use super::model::RegisterRequestDto;

pub struct AuthService;

impl AuthService {
    #[instrument]
    pub async fn register_user(db: &PgPool, dto: RegisterRequestDto) -> Result<(), AppError> {
        Ok(())
    }
}
