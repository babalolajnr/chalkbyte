use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::utils::errors::AppError;

pub async fn get_admin_school_id(db: &PgPool, auth_user: &AuthUser) -> Result<Uuid, AppError> {
    let user_id = Uuid::parse_str(&auth_user.0.sub)
        .map_err(|_| AppError::unauthorized("Invalid user ID".to_string()))?;

    let school_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await?
            .ok_or_else(|| {
                AppError::forbidden("Admin must be associated with a school".to_string())
            })?;

    Ok(school_id)
}
