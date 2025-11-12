use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::config::jwt::JwtConfig;
use crate::modules::users::model::User;
use crate::utils::errors::AppError;
use crate::utils::jwt::create_access_token;
use crate::utils::password::{hash_password, verify_password};

use super::model::{LoginRequest, LoginResponse, RegisterRequestDto};

pub struct AuthService;

impl AuthService {
    #[instrument]
    pub async fn register_user(db: &PgPool, dto: RegisterRequestDto) -> Result<User, AppError> {
        let hashed_password = hash_password(&dto.password)?;
        let role = dto.role.unwrap_or_default();

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (first_name, last_name, email, password, role) 
             VALUES ($1, $2, $3, $4, $5) 
             ON CONFLICT (email) DO NOTHING
             RETURNING id, first_name, last_name, email, role",
        )
        .bind(&dto.first_name)
        .bind(&dto.last_name)
        .bind(&dto.email)
        .bind(&hashed_password)
        .bind(&role)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::bad_request(anyhow::anyhow!("Email already exists")))?;

        Ok(user)
    }

    #[instrument]
    pub async fn login_user(
        db: &PgPool,
        dto: LoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        use crate::modules::users::model::UserRole;

        #[derive(sqlx::FromRow)]
        struct UserWithPassword {
            id: Uuid,
            first_name: String,
            last_name: String,
            email: String,
            password: String,
            role: UserRole,
        }

        let user_with_password = sqlx::query_as::<_, UserWithPassword>(
            "SELECT id, first_name, last_name, email, password, role FROM users WHERE email = $1",
        )
        .bind(&dto.email)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("Invalid email or password".to_string()))?;

        let is_valid = verify_password(&dto.password, &user_with_password.password)?;

        if !is_valid {
            return Err(AppError::unauthorized(
                "Invalid email or password".to_string(),
            ));
        }

        let access_token =
            create_access_token(user_with_password.id, &user_with_password.email, &user_with_password.role, jwt_config)?;

        let user = User {
            id: user_with_password.id,
            first_name: user_with_password.first_name,
            last_name: user_with_password.last_name,
            email: user_with_password.email,
            role: user_with_password.role,
        };

        Ok(LoginResponse { access_token, user })
    }
}
