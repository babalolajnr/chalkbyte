use crate::{
    modules::users::model::{CreateUserDto, School, User, UserRole, UserWithSchool},
    utils::errors::AppError,
};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserService;

impl UserService {
    pub async fn create_user(db: &PgPool, dto: CreateUserDto) -> Result<User, AppError> {
        let role = dto.role.unwrap_or_default();
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (first_name, last_name, email, role, school_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, first_name, last_name, email, role as "role: _", school_id
            "#,
            dto.first_name,
            dto.last_name,
            dto.email,
            role as _,
            dto.school_id
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
            SELECT id, first_name, last_name, email, role as "role: _", school_id
            FROM users
            "#
        )
        .fetch_all(db)
        .await
        .context("Failed to fetch users")
        .map_err(AppError::database)?;

        Ok(users)
    }

    pub async fn get_users_by_school(db: &PgPool, school_id: Uuid) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, first_name, last_name, email, role as "role: _", school_id
            FROM users
            WHERE school_id = $1
            "#,
            school_id
        )
        .fetch_all(db)
        .await
        .context("Failed to fetch users by school")
        .map_err(AppError::database)?;

        Ok(users)
    }

    pub async fn get_user(db: &PgPool, id: Uuid) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, first_name, last_name, email, role as "role: _", school_id
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

    pub async fn get_user_with_school(db: &PgPool, id: Uuid) -> Result<UserWithSchool, AppError> {
        let result = sqlx::query!(
            r#"
                SELECT
                    users.id as user_id,
                    users.first_name,
                    users.last_name,
                    users.email,
                    users.role as "role: UserRole",
                    users.school_id,
                    s.id as "school_id_inner?",
                    s.name as "school_name?",
                    s.address as "school_address?"
                FROM users
                LEFT JOIN schools s ON users.school_id = s.id
                WHERE users.id = $1
                "#,
            id
        )
        .fetch_optional(db)
        .await
        .context("Failed to fetch user with school by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("User with id {} not found", id)))?;

        let user = User {
            id: result.user_id,
            first_name: result.first_name,
            last_name: result.last_name,
            email: result.email,
            role: result.role,
            school_id: result.school_id,
        };

        let school = if let (Some(school_id), Some(school_name)) =
            (result.school_id_inner, result.school_name)
        {
            Some(School {
                id: school_id,
                name: school_name,
                address: result.school_address,
            })
        } else {
            None
        };

        Ok(UserWithSchool { user, school })
    }
}
