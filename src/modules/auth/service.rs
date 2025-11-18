use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::config::email::EmailConfig;
use crate::config::jwt::JwtConfig;
use crate::modules::users::model::User;
use crate::utils::email::EmailService;
use crate::utils::errors::AppError;
use crate::utils::jwt::{
    create_access_token, create_mfa_temp_token, create_refresh_token, verify_mfa_temp_token,
    verify_refresh_token,
};
use crate::utils::password::{hash_password, verify_password};

use super::model::{
    ForgotPasswordRequest, LoginRequest, LoginResponse, MfaRecoveryLoginRequest,
    MfaRequiredResponse, MfaVerifyLoginRequest, RefreshTokenRequest, RegisterRequestDto,
    ResetPasswordRequest,
};

pub struct AuthService;

impl AuthService {
    #[instrument]
    pub async fn register_user(db: &PgPool, dto: RegisterRequestDto) -> Result<User, AppError> {
        let hashed_password = hash_password(&dto.password)?;
        let role = dto.role.unwrap_or_default();

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (first_name, last_name, email, password, role, school_id)
             VALUES ($1, $2, $3, $4, $5, NULL)
             ON CONFLICT (email) DO NOTHING
             RETURNING id, first_name, last_name, email, role, school_id",
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
    ) -> Result<Result<LoginResponse, MfaRequiredResponse>, AppError> {
        use crate::modules::users::model::UserRole;

        #[derive(sqlx::FromRow)]
        struct UserWithPassword {
            id: Uuid,
            first_name: String,
            last_name: String,
            email: String,
            password: String,
            role: UserRole,
            school_id: Option<Uuid>,
            mfa_enabled: bool,
        }

        let user_with_password = sqlx::query_as::<_, UserWithPassword>(
            "SELECT id, first_name, last_name, email, password, role, school_id, mfa_enabled FROM users WHERE email = $1",
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

        // Check if MFA is enabled
        if user_with_password.mfa_enabled {
            // Generate temporary token for MFA verification
            let temp_token = create_mfa_temp_token(
                user_with_password.id,
                &user_with_password.email,
                &user_with_password.role,
                jwt_config,
            )?;

            return Ok(Err(MfaRequiredResponse {
                mfa_required: true,
                temp_token,
            }));
        }

        // No MFA, proceed with normal login
        let access_token = create_access_token(
            user_with_password.id,
            &user_with_password.email,
            &user_with_password.role,
            jwt_config,
        )?;

        let refresh_token = create_refresh_token(
            user_with_password.id,
            &user_with_password.email,
            &user_with_password.role,
            jwt_config,
        )?;

        // Store refresh token in database
        let expires_at = Utc::now() + Duration::seconds(jwt_config.refresh_token_expiry);
        sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_with_password.id)
            .bind(&refresh_token)
            .bind(expires_at)
            .execute(db)
            .await?;

        let user = User {
            id: user_with_password.id,
            first_name: user_with_password.first_name,
            last_name: user_with_password.last_name,
            email: user_with_password.email,
            role: user_with_password.role,
            school_id: user_with_password.school_id,
        };

        Ok(Ok(LoginResponse {
            access_token,
            refresh_token,
            user,
        }))
    }

    #[instrument]
    pub async fn verify_mfa_login(
        db: &PgPool,
        dto: MfaVerifyLoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        use crate::modules::mfa::service::MfaService;

        // Verify temp token
        let temp_claims = verify_mfa_temp_token(&dto.temp_token, jwt_config)?;

        let user_id = Uuid::parse_str(&temp_claims.sub)
            .map_err(|_| AppError::unauthorized("Invalid token".to_string()))?;

        // Verify TOTP code
        let is_valid = MfaService::verify_totp_login(db, user_id, &dto.code).await?;

        if !is_valid {
            return Err(AppError::unauthorized("Invalid MFA code".to_string()));
        }

        // Get user details
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // Generate final access token
        let access_token = create_access_token(user_id, &user.email, &user.role, jwt_config)?;

        let refresh_token = create_refresh_token(user_id, &user.email, &user.role, jwt_config)?;

        // Store refresh token in database
        let expires_at = Utc::now() + Duration::seconds(jwt_config.refresh_token_expiry);
        sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&refresh_token)
            .bind(expires_at)
            .execute(db)
            .await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            user,
        })
    }

    #[instrument]
    pub async fn verify_mfa_recovery_login(
        db: &PgPool,
        dto: MfaRecoveryLoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        use crate::modules::mfa::service::MfaService;

        // Verify temp token
        let temp_claims = verify_mfa_temp_token(&dto.temp_token, jwt_config)?;

        let user_id = Uuid::parse_str(&temp_claims.sub)
            .map_err(|_| AppError::unauthorized("Invalid token".to_string()))?;

        // Verify recovery code
        let is_valid =
            MfaService::verify_recovery_code_login(db, user_id, &dto.recovery_code).await?;

        if !is_valid {
            return Err(AppError::unauthorized(
                "Invalid or already used recovery code".to_string(),
            ));
        }

        // Get user details
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // Generate final access token
        let access_token = create_access_token(user_id, &user.email, &user.role, jwt_config)?;

        let refresh_token = create_refresh_token(user_id, &user.email, &user.role, jwt_config)?;

        // Store refresh token in database
        let expires_at = Utc::now() + Duration::seconds(jwt_config.refresh_token_expiry);
        sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&refresh_token)
            .bind(expires_at)
            .execute(db)
            .await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            user,
        })
    }

    #[instrument]
    pub async fn forgot_password(
        db: &PgPool,
        dto: ForgotPasswordRequest,
        email_config: &EmailConfig,
    ) -> Result<(), AppError> {
        // Find user by email
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE email = $1",
        )
        .bind(&dto.email)
        .fetch_optional(db)
        .await?;

        // Always return success to prevent email enumeration
        if user.is_none() {
            return Ok(());
        }

        let user = user.unwrap();

        // Generate reset token (using UUID for simplicity and security)
        let reset_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(1);

        // Delete any existing unused tokens for this user
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1 AND used = FALSE")
            .bind(user.id)
            .execute(db)
            .await?;

        // Store reset token in database
        sqlx::query(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(user.id)
        .bind(&reset_token)
        .bind(expires_at)
        .execute(db)
        .await?;

        // Send email
        let email_service = EmailService::new(email_config.clone());
        email_service
            .send_password_reset_email(&user.email, &user.first_name, &reset_token)
            .await?;

        Ok(())
    }

    #[instrument]
    pub async fn reset_password(
        db: &PgPool,
        dto: ResetPasswordRequest,
        email_config: &EmailConfig,
    ) -> Result<(), AppError> {
        // Find and validate token
        #[derive(sqlx::FromRow)]
        struct ResetToken {
            id: Uuid,
            user_id: Uuid,
            expires_at: chrono::DateTime<Utc>,
            used: bool,
        }

        let token_record = sqlx::query_as::<_, ResetToken>(
            "SELECT id, user_id, expires_at, used FROM password_reset_tokens WHERE token = $1",
        )
        .bind(&dto.token)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::bad_request(anyhow::anyhow!("Invalid or expired reset token")))?;

        // Check if token is already used
        if token_record.used {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Reset token has already been used"
            )));
        }

        // Check if token is expired
        if token_record.expires_at < Utc::now() {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Reset token has expired"
            )));
        }

        // Get user details
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE id = $1",
        )
        .bind(token_record.user_id)
        .fetch_one(db)
        .await?;

        // Hash new password
        let hashed_password = hash_password(&dto.new_password)?;

        // Update password
        sqlx::query("UPDATE users SET password = $1, updated_at = NOW() WHERE id = $2")
            .bind(&hashed_password)
            .bind(token_record.user_id)
            .execute(db)
            .await?;

        // Mark token as used
        sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE id = $1")
            .bind(token_record.id)
            .execute(db)
            .await?;

        // Send confirmation email
        let email_service = EmailService::new(email_config.clone());
        email_service
            .send_password_reset_confirmation(&user.email, &user.first_name)
            .await?;

        Ok(())
    }

    #[instrument]
    pub async fn refresh_access_token(
        db: &PgPool,
        dto: RefreshTokenRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        // Verify refresh token JWT signature and expiry
        let claims = verify_refresh_token(&dto.refresh_token, jwt_config)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::unauthorized("Invalid token".to_string()))?;

        // Check if refresh token exists in database and is not revoked
        #[derive(sqlx::FromRow)]
        struct RefreshTokenRecord {
            revoked: bool,
            expires_at: chrono::DateTime<Utc>,
        }

        let token_record = sqlx::query_as::<_, RefreshTokenRecord>(
            "SELECT revoked, expires_at FROM refresh_tokens WHERE token = $1 AND user_id = $2",
        )
        .bind(&dto.refresh_token)
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("Invalid refresh token".to_string()))?;

        if token_record.revoked {
            return Err(AppError::unauthorized(
                "Refresh token has been revoked".to_string(),
            ));
        }

        if token_record.expires_at < Utc::now() {
            return Err(AppError::unauthorized(
                "Refresh token has expired".to_string(),
            ));
        }

        // Get user details
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        // Generate new access token
        let access_token = create_access_token(user_id, &user.email, &user.role, jwt_config)?;

        // Generate new refresh token (refresh token rotation)
        let new_refresh_token = create_refresh_token(user_id, &user.email, &user.role, jwt_config)?;

        // Revoke old refresh token
        sqlx::query(
            "UPDATE refresh_tokens SET revoked = TRUE, updated_at = NOW() WHERE token = $1",
        )
        .bind(&dto.refresh_token)
        .execute(db)
        .await?;

        // Store new refresh token
        let expires_at = Utc::now() + Duration::seconds(jwt_config.refresh_token_expiry);
        sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&new_refresh_token)
            .bind(expires_at)
            .execute(db)
            .await?;

        Ok(LoginResponse {
            access_token,
            refresh_token: new_refresh_token,
            user,
        })
    }

    #[instrument]
    pub async fn revoke_refresh_token(
        db: &PgPool,
        user_id: Uuid,
        refresh_token: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked = TRUE, updated_at = NOW()
             WHERE token = $1 AND user_id = $2 AND revoked = FALSE",
        )
        .bind(refresh_token)
        .bind(user_id)
        .execute(db)
        .await?;

        Ok(())
    }

    #[instrument]
    pub async fn revoke_all_refresh_tokens(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked = TRUE, updated_at = NOW()
             WHERE user_id = $1 AND revoked = FALSE",
        )
        .bind(user_id)
        .execute(db)
        .await?;

        Ok(())
    }
}
