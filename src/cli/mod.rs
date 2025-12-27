use crate::modules::users::model::system_roles;
use crate::utils::password::hash_password;
use sqlx::PgPool;

pub mod seeder;

pub async fn create_system_admin(
    db: &PgPool,
    first_name: &str,
    last_name: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hashed_password =
        hash_password(password).map_err(|e| format!("Failed to hash password: {}", e.error))?;

    // Start a transaction
    let mut tx = db.begin().await?;

    // Insert the user
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO users (first_name, last_name, email, password, school_id)
         VALUES ($1, $2, $3, $4, NULL)
         ON CONFLICT (email) DO NOTHING
         RETURNING id",
    )
    .bind(first_name)
    .bind(last_name)
    .bind(email)
    .bind(&hashed_password)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = match user_id {
        Some(id) => id,
        None => {
            tx.rollback().await?;
            return Err("User with this email already exists".into());
        }
    };

    // Assign the system admin role
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id, role_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(system_roles::SYSTEM_ADMIN)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}
