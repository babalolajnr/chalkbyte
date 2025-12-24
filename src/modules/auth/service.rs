use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::config::email::EmailConfig;
use crate::config::jwt::JwtConfig;
use crate::metrics;
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
    MfaRequiredResponse, MfaVerifyLoginRequest, RefreshTokenRequest, ResetPasswordRequest,
};

pub struct AuthService;

impl AuthService {
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
            level_id: Option<Uuid>,
            branch_id: Option<Uuid>,
            date_of_birth: Option<chrono::NaiveDate>,
            grade_level: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            mfa_enabled: bool,
        }

        let user_with_password = sqlx::query_as::<_, UserWithPassword>(
            "SELECT id, first_name, last_name, email, password, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at, mfa_enabled FROM users WHERE email = $1",
        )
        .bind(&dto.email)
        .fetch_optional(db)
        .await?
        .ok_or_else(||{
            metrics::track_user_login_failure("invalid_email");
            AppError::unauthorized("Invalid email or password".to_string() )
        })?;

        let is_valid = verify_password(&dto.password, &user_with_password.password)?;

        if !is_valid {
            metrics::track_user_login_failure("invalid_password");
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

            metrics::track_jwt_issued();
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

        // Track metrics
        metrics::track_jwt_issued();
        metrics::track_user_login_success(&user_with_password.role.to_string());

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
            level_id: user_with_password.level_id,
            branch_id: user_with_password.branch_id,
            date_of_birth: user_with_password.date_of_birth,
            grade_level: user_with_password.grade_level,
            created_at: user_with_password.created_at,
            updated_at: user_with_password.updated_at,
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
            metrics::track_user_login_failure("invalid_mfa_code");
            return Err(AppError::unauthorized("Invalid MFA code".to_string()));
        }

        // Get user details
        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1",
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
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1",
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
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE email = $1",
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
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1",
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
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::jwt::JwtConfig;
    use crate::modules::users::model::UserRole;
    use chrono::Utc;
    use sqlx::PgPool;

    async fn create_test_user(db: &PgPool, email: &str, password: &str, mfa_enabled: bool) -> Uuid {
        let user_id = Uuid::new_v4();
        let hashed_password = hash_password(password).unwrap();

        sqlx::query(
            "INSERT INTO users (id, first_name, last_name, email, password, role, mfa_enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
        )
        .bind(user_id)
        .bind("Test")
        .bind("User")
        .bind(email)
        .bind(hashed_password)
        .bind(UserRole::Student)
        .bind(mfa_enabled)
        .execute(db)
        .await
        .unwrap();

        user_id
    }

    async fn cleanup_test_user(db: &PgPool, user_id: Uuid) {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await
            .ok();

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(db)
            .await
            .ok();
    }

    #[sqlx::test]
    async fn test_login_user_returns_all_user_fields(pool: PgPool) {
        let email = format!("test_login_{}@example.com", Uuid::new_v4());
        let password = "testpassword123";
        let user_id = create_test_user(&pool, &email, password, false).await;

        let jwt_config = JwtConfig {
            secret: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
        };

        let login_dto = LoginRequest {
            email: email.clone(),
            password: password.to_string(),
        };

        let result = AuthService::login_user(&pool, login_dto, &jwt_config).await;

        assert!(result.is_ok());
        let login_result = result.unwrap();

        match login_result {
            Ok(response) => {
                assert_eq!(response.user.email, email);
                assert_eq!(response.user.first_name, "Test");
                assert_eq!(response.user.last_name, "User");
                assert_eq!(response.user.role, UserRole::Student);
                assert!(response.user.created_at <= Utc::now());
                assert!(response.user.updated_at <= Utc::now());
            }
            Err(_) => panic!("Expected successful login without MFA"),
        }

        cleanup_test_user(&pool, user_id).await;
    }

    #[sqlx::test]
    async fn test_verify_mfa_login_returns_all_user_fields(pool: PgPool) {
        let email = format!("test_mfa_{}@example.com", Uuid::new_v4());
        let password = "testpassword123";
        let user_id = create_test_user(&pool, &email, password, false).await;

        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        assert!(
            user.is_ok(),
            "User query should return all fields without error: {:?}",
            user.as_ref().err()
        );
        let user = user.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.email, email);
        assert!(user.created_at <= Utc::now());
        assert!(user.updated_at <= Utc::now());

        cleanup_test_user(&pool, user_id).await;
    }

    #[sqlx::test]
    async fn test_forgot_password_returns_all_user_fields(pool: PgPool) {
        let email = format!("test_forgot_{}@example.com", Uuid::new_v4());
        let password = "testpassword123";
        let user_id = create_test_user(&pool, &email, password, false).await;

        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(&email)
        .fetch_optional(&pool)
        .await;

        assert!(
            user.is_ok(),
            "User query should return all fields without error"
        );
        let user = user.unwrap();
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, email);
        assert!(user.created_at <= Utc::now());
        assert!(user.updated_at <= Utc::now());

        cleanup_test_user(&pool, user_id).await;
    }

    #[sqlx::test]
    async fn test_refresh_access_token_returns_all_user_fields(pool: PgPool) {
        let email = format!("test_refresh_{}@example.com", Uuid::new_v4());
        let password = "testpassword123";
        let user_id = create_test_user(&pool, &email, password, false).await;

        let user = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        assert!(
            user.is_ok(),
            "User query for refresh token flow should return all fields without error: {:?}",
            user.as_ref().err()
        );
        let user = user.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.id, user_id);
        assert!(user.created_at <= Utc::now());
        assert!(user.updated_at <= Utc::now());

        cleanup_test_user(&pool, user_id).await;
    }

    #[sqlx::test]
    async fn test_user_query_includes_all_required_fields(pool: PgPool) {
        let email = format!("test_fields_{}@example.com", Uuid::new_v4());
        let password = "testpassword123";
        let user_id = create_test_user(&pool, &email, password, false).await;

        let query_result = sqlx::query_as::<_, User>(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        assert!(
            query_result.is_ok(),
            "Query with all User fields should succeed: {:?}",
            query_result.err()
        );

        let user = query_result.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.email, email);
        assert_eq!(user.first_name, "Test");
        assert_eq!(user.last_name, "User");
        assert_eq!(user.role, UserRole::Student);
        assert_eq!(user.school_id, None);
        assert_eq!(user.level_id, None);
        assert_eq!(user.branch_id, None);
        assert_eq!(user.date_of_birth, None);
        assert_eq!(user.grade_level, None);
        assert!(user.created_at <= Utc::now());
        assert!(user.updated_at <= Utc::now());

        cleanup_test_user(&pool, user_id).await;
    }
}
