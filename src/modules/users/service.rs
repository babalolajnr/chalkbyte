use crate::{
    modules::users::model::{CreateUserDto, User},
    utils::errors::AppError,
};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserService;

impl UserService {
    pub async fn create_user(db: &PgPool, dto: CreateUserDto) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (first_name, last_name, email)
            VALUES ($1, $2, $3)
            RETURNING id, first_name, last_name, email
            "#,
            dto.first_name,
            dto.last_name,
            dto.email
        )
        .fetch_one(db)
        .await
        .context("Failed to insert user")
        .map_err(AppError::database)?;

        Ok(user)
    }

    pub async fn get_users(db: &PgPool) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, first_name, last_name, email
            FROM users
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed to fetch users")
        .map_err(AppError::database)?;

        Ok(users)
    }

    pub async fn get_user(db: &PgPool, id: Uuid) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, first_name, last_name, email
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(db)
        .await
        .context("Failed to fetch user by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("User with id {} not found", id)))?;

        Ok(user)
    }
}
