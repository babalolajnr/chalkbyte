use sqlx::PgPool;
use crate::modules::users::model::UserRole;
use crate::utils::password::hash_password;

pub async fn create_system_admin(
    db: &PgPool,
    first_name: &str,
    last_name: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hashed_password = hash_password(password)
        .map_err(|e| format!("Failed to hash password: {}", e.error))?;

    let result = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, role, school_id) 
         VALUES ($1, $2, $3, $4, $5, NULL)
         ON CONFLICT (email) DO NOTHING"
    )
    .bind(first_name)
    .bind(last_name)
    .bind(email)
    .bind(hashed_password)
    .bind(UserRole::SystemAdmin)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err("User with this email already exists".into());
    }

    Ok(())
}
