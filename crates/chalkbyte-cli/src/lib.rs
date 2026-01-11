//! # Chalkbyte CLI
//!
//! Command-line interface utilities for Chalkbyte administration and database seeding.
//!
//! This crate provides:
//!
//! - **System Admin Creation**: Create system administrators via CLI
//! - **Database Seeding**: Populate the database with fake test data
//!
//! ## Usage
//!
//! ### Creating a System Admin
//!
//! ```ignore
//! use chalkbyte_cli::create_system_admin;
//!
//! create_system_admin(&pool, "John", "Doe", "john@example.com", "password123").await?;
//! ```
//!
//! ### Seeding the Database
//!
//! ```ignore
//! use chalkbyte_cli::seeder::{seed_all, SeedConfig};
//!
//! let config = SeedConfig::new(10); // 10 schools with defaults
//! seed_all(&pool, config).await?;
//! ```

pub mod seeder;

use chalkbyte_core::hash_password;
use chalkbyte_models::users::system_roles;
use sqlx::PgPool;

/// Creates a system administrator account.
///
/// System admins have full access to all system operations and are not
/// scoped to any specific school. They can only be created via CLI,
/// never through the API.
///
/// # Arguments
///
/// * `db` - PostgreSQL connection pool
/// * `first_name` - Admin's first name
/// * `last_name` - Admin's last name
/// * `email` - Admin's email address (must be unique)
/// * `password` - Plain text password (will be hashed)
///
/// # Errors
///
/// Returns an error if:
/// - Password hashing fails
/// - A user with the same email already exists
/// - Database operations fail
///
/// # Example
///
/// ```ignore
/// let pool = sqlx::PgPool::connect(&database_url).await?;
/// create_system_admin(&pool, "Admin", "User", "admin@example.com", "SecurePass123!").await?;
/// ```
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
