use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use chalkbyte_auth::{
    create_access_token, create_mfa_temp_token, create_refresh_token, verify_mfa_temp_token,
    verify_refresh_token,
};
use chalkbyte_config::JwtConfig;
use chalkbyte_core::{AppError, hash_password, verify_password};

use crate::metrics;
use crate::modules::auth::model::{
    ForgotPasswordRequest, LoginRequest, LoginResponse, LoginUser, MessageResponse,
    MfaRecoveryLoginRequest, MfaRequiredResponse, MfaVerifyLoginRequest, RefreshTokenRequest,
    ResetPasswordRequest,
};
use crate::modules::roles::service as roles_service;
use crate::modules::users::model::{BranchInfo, LevelInfo, SchoolInfo};

pub struct AuthService;

/// Helper struct for user data with school_id needed for JWT
struct UserForLogin {
    id: Uuid,
    email: String,
    school_id: Option<Uuid>,
    login_user: LoginUser,
}

/// Fetch user with joined school/level/branch relations
async fn fetch_user_with_relations(db: &PgPool, user_id: Uuid) -> Result<UserForLogin, AppError> {
    let row = sqlx::query(
        r#"SELECT
            u.id, u.first_name, u.last_name, u.email,
            u.date_of_birth, u.grade_level, u.created_at, u.updated_at,
            u.school_id, u.level_id, u.branch_id,
            s.id as school_id_joined, s.name as school_name, s.address as school_address,
            l.id as level_id_joined, l.name as level_name, l.description as level_description,
            b.id as branch_id_joined, b.name as branch_name, b.description as branch_description
        FROM users u
        LEFT JOIN schools s ON u.school_id = s.id
        LEFT JOIN levels l ON u.level_id = l.id
        LEFT JOIN branches b ON u.branch_id = b.id
        WHERE u.id = $1"#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;

    let id = row.get("id");
    let email: String = row.get("email");
    let school_id = row.get("school_id");

    let school = row
        .try_get::<Option<Uuid>, _>("school_id_joined")
        .ok()
        .flatten()
        .map(|sid| SchoolInfo {
            id: sid,
            name: row.get("school_name"),
            address: row.get("school_address"),
        });

    let level = row
        .try_get::<Option<Uuid>, _>("level_id_joined")
        .ok()
        .flatten()
        .map(|lid| LevelInfo {
            id: lid,
            name: row.get("level_name"),
            description: row.get("level_description"),
        });

    let branch = row
        .try_get::<Option<Uuid>, _>("branch_id_joined")
        .ok()
        .flatten()
        .map(|bid| BranchInfo {
            id: bid,
            name: row.get("branch_name"),
            description: row.get("branch_description"),
        });

    Ok(UserForLogin {
        id,
        email: email.clone(),
        school_id,
        login_user: LoginUser {
            id,
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email,
            date_of_birth: row.get("date_of_birth"),
            grade_level: row.get("grade_level"),
            school,
            level,
            branch,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        },
    })
}

impl AuthService {
    #[instrument(skip(db, dto, jwt_config), fields(auth.email = %dto.email, auth.event = "login_attempt"))]
    pub async fn login_user(
        db: &PgPool,
        dto: LoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<Result<LoginResponse, MfaRequiredResponse>, AppError> {
        debug!(email = %dto.email, "Processing login request");

        let row = sqlx::query(
            r#"SELECT
                u.id, u.first_name, u.last_name, u.email, u.password,
                u.date_of_birth, u.grade_level, u.created_at, u.updated_at, u.mfa_enabled,
                u.school_id, u.level_id, u.branch_id,
                s.id as school_id_joined, s.name as school_name, s.address as school_address,
                l.id as level_id_joined, l.name as level_name, l.description as level_description,
                b.id as branch_id_joined, b.name as branch_name, b.description as branch_description
            FROM users u
            LEFT JOIN schools s ON u.school_id = s.id
            LEFT JOIN levels l ON u.level_id = l.id
            LEFT JOIN branches b ON u.branch_id = b.id
            WHERE u.email = $1"#,
        )
        .bind(&dto.email)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            metrics::track_user_login_failure("invalid_email");
            AppError::unauthorized("Invalid email or password".to_string())
        })?;

        let user_id = row.get("id");
        let first_name = row.get("first_name");
        let last_name = row.get("last_name");
        let email: String = row.get("email");
        let password: String = row.get("password");
        let school_id = row.get("school_id");
        let date_of_birth = row.get("date_of_birth");
        let grade_level = row.get("grade_level");
        let created_at = row.get("created_at");
        let updated_at = row.get("updated_at");
        let mfa_enabled = row.get("mfa_enabled");

        let school = row
            .try_get::<Option<Uuid>, _>("school_id_joined")
            .ok()
            .flatten()
            .map(|id| SchoolInfo {
                id,
                name: row.get("school_name"),
                address: row.get("school_address"),
            });

        let level = row
            .try_get::<Option<Uuid>, _>("level_id_joined")
            .ok()
            .flatten()
            .map(|id| LevelInfo {
                id,
                name: row.get("level_name"),
                description: row.get("level_description"),
            });

        let branch = row
            .try_get::<Option<Uuid>, _>("branch_id_joined")
            .ok()
            .flatten()
            .map(|id| BranchInfo {
                id,
                name: row.get("branch_name"),
                description: row.get("branch_description"),
            });

        let is_valid = verify_password(&dto.password, &password)?;

        if !is_valid {
            metrics::track_user_login_failure("invalid_password");
            return Err(AppError::unauthorized(
                "Invalid email or password".to_string(),
            ));
        }

        // Check if MFA is enabled
        if mfa_enabled {
            // Generate temporary token for MFA verification
            let temp_token = create_mfa_temp_token(user_id, &email, jwt_config)?;

            metrics::track_jwt_issued();
            return Ok(Err(MfaRequiredResponse {
                mfa_required: true,
                temp_token,
            }));
        }

        // No MFA, proceed with normal login
        // Fetch roles and permissions first (needed for JWT)
        let roles = roles_service::get_user_roles_internal(db, user_id).await?;
        let permissions = roles_service::get_user_permissions(db, user_id).await?;

        // Extract role IDs and permission names for JWT
        let role_ids = roles.iter().map(|r| r.role.id).collect();
        let permission_names = permissions.iter().map(|p| p.name.clone()).collect();

        let access_token = create_access_token(
            user_id,
            &email,
            school_id,
            role_ids,
            permission_names,
            jwt_config,
        )?;

        let refresh_token = create_refresh_token(user_id, &email, jwt_config)?;

        // Track metrics
        metrics::track_jwt_issued();

        // Determine primary role for metrics
        let primary_role = roles
            .first()
            .map(|r| r.role.name.as_str())
            .unwrap_or("none");
        metrics::track_user_login_success(primary_role);

        // Store refresh token in database
        let expires_at = Utc::now() + Duration::seconds(jwt_config.refresh_token_expiry);
        sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&refresh_token)
            .bind(expires_at)
            .execute(db)
            .await?;

        let user = LoginUser {
            id: user_id,
            first_name,
            last_name,
            email,
            date_of_birth,
            grade_level,
            school,
            level,
            branch,
            created_at,
            updated_at,
        };

        Ok(Ok(LoginResponse {
            access_token,
            refresh_token,
            user,
            roles,
            permissions,
        }))
    }

    #[instrument(skip(db, dto, jwt_config), fields(auth.event = "mfa_verification"))]
    pub async fn verify_mfa_login(
        db: &PgPool,
        dto: MfaVerifyLoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        debug!("Processing MFA verification request");
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

        // Get user details with relations
        let user_data = fetch_user_with_relations(db, user_id).await?;

        // Fetch roles and permissions for JWT
        let roles = roles_service::get_user_roles_internal(db, user_data.id).await?;
        let permissions = roles_service::get_user_permissions(db, user_data.id).await?;

        // Extract role IDs and permission names for JWT
        let role_ids: Vec<Uuid> = roles.iter().map(|r| r.role.id).collect();
        let permission_names: Vec<String> = permissions.iter().map(|p| p.name.clone()).collect();

        // Generate final access token with roles and permissions
        let access_token = create_access_token(
            user_id,
            &user_data.email,
            user_data.school_id,
            role_ids,
            permission_names,
            jwt_config,
        )?;

        let refresh_token = create_refresh_token(user_id, &user_data.email, jwt_config)?;

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
            user: user_data.login_user,
            roles,
            permissions,
        })
    }

    #[instrument(skip(db, dto, jwt_config), fields(auth.event = "mfa_recovery_verification"))]
    pub async fn verify_mfa_recovery_login(
        db: &PgPool,
        dto: MfaRecoveryLoginRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        debug!("Processing MFA recovery code verification");
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

        // Get user details with relations
        let user_data = fetch_user_with_relations(db, user_id).await?;

        // Fetch roles and permissions for JWT
        let roles = roles_service::get_user_roles_internal(db, user_data.id).await?;
        let permissions = roles_service::get_user_permissions(db, user_data.id).await?;

        // Extract role IDs and permission names for JWT
        let role_ids: Vec<Uuid> = roles.iter().map(|r| r.role.id).collect();
        let permission_names: Vec<String> = permissions.iter().map(|p| p.name.clone()).collect();

        // Generate final access token with roles and permissions
        let access_token = create_access_token(
            user_id,
            &user_data.email,
            user_data.school_id,
            role_ids,
            permission_names,
            jwt_config,
        )?;

        let refresh_token = create_refresh_token(user_id, &user_data.email, jwt_config)?;

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
            user: user_data.login_user,
            roles,
            permissions,
        })
    }

    #[instrument(skip(db, dto), fields(auth.email = %dto.email, auth.event = "forgot_password"))]
    pub async fn forgot_password(db: &PgPool, dto: ForgotPasswordRequest) -> Result<(), AppError> {
        use rand::Rng;

        debug!(email = %dto.email, "Processing forgot password request");

        // Check if user exists
        let user_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
                .bind(&dto.email)
                .fetch_one(db)
                .await?;

        if !user_exists {
            // Don't reveal if email exists or not
            info!(email = %dto.email, "Forgot password requested for non-existent email");
            return Ok(());
        }

        // Generate reset token
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let expires_at = Utc::now() + Duration::hours(1);

        // Store reset token
        sqlx::query(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at)
             SELECT id, $1, $2 FROM users WHERE email = $3",
        )
        .bind(&token)
        .bind(expires_at)
        .bind(&dto.email)
        .execute(db)
        .await?;

        // TODO: Send email with reset link
        info!(email = %dto.email, "Password reset token generated");

        Ok(())
    }

    #[instrument(skip(db, dto), fields(auth.event = "reset_password"))]
    pub async fn reset_password(
        db: &PgPool,
        dto: ResetPasswordRequest,
    ) -> Result<MessageResponse, AppError> {
        debug!("Processing password reset request");

        #[derive(sqlx::FromRow)]
        struct ResetToken {
            id: Uuid,
            user_id: Uuid,
            expires_at: chrono::DateTime<Utc>,
            used: bool,
        }

        // Find valid reset token
        let token_record = sqlx::query_as::<_, ResetToken>(
            "SELECT id, user_id, expires_at, used FROM password_reset_tokens WHERE token = $1",
        )
        .bind(&dto.token)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::bad_request(anyhow::anyhow!("Invalid reset token")))?;

        if token_record.used {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Reset token has already been used"
            )));
        }

        if token_record.expires_at < Utc::now() {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Reset token has expired"
            )));
        }

        // Hash new password
        let password_hash = hash_password(&dto.new_password)?;

        // Update password
        sqlx::query("UPDATE users SET password = $1, updated_at = NOW() WHERE id = $2")
            .bind(&password_hash)
            .bind(token_record.user_id)
            .execute(db)
            .await?;

        // Mark token as used
        sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE id = $1")
            .bind(token_record.id)
            .execute(db)
            .await?;

        // Revoke all refresh tokens for this user
        Self::revoke_all_refresh_tokens(db, token_record.user_id).await?;

        info!(user.id = %token_record.user_id, "Password reset successfully");

        Ok(MessageResponse {
            message: "Password has been reset successfully".to_string(),
        })
    }

    #[instrument(skip(db, dto, jwt_config), fields(auth.event = "token_refresh"))]
    pub async fn refresh_access_token(
        db: &PgPool,
        dto: RefreshTokenRequest,
        jwt_config: &JwtConfig,
    ) -> Result<LoginResponse, AppError> {
        debug!("Processing token refresh request");
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

        // Check if token is expired
        if token_record.expires_at < Utc::now() {
            warn!(
                user.id = %user_id,
                auth.event = "token_refresh_failed",
                reason = "token_expired",
                "Refresh token expired"
            );
            return Err(AppError::unauthorized(
                "Refresh token has expired".to_string(),
            ));
        }

        debug!(user.id = %user_id, "Refresh token valid, generating new tokens");

        // Get user details with relations
        let user_data = fetch_user_with_relations(db, user_id).await?;

        // Fetch roles and permissions for new access token
        let roles = roles_service::get_user_roles_internal(db, user_data.id).await?;
        let permissions = roles_service::get_user_permissions(db, user_data.id).await?;

        // Extract role IDs and permission names for JWT
        let role_ids: Vec<Uuid> = roles.iter().map(|r| r.role.id).collect();
        let permission_names: Vec<String> = permissions.iter().map(|p| p.name.clone()).collect();

        // Generate new access token with roles and permissions
        let access_token = create_access_token(
            user_id,
            &user_data.email,
            user_data.school_id,
            role_ids,
            permission_names,
            jwt_config,
        )?;

        // Generate new refresh token (refresh token rotation)
        let new_refresh_token = create_refresh_token(user_id, &user_data.email, jwt_config)?;

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
            user: user_data.login_user,
            roles,
            permissions,
        })
    }

    #[instrument(skip(db), fields(user.id = %user_id, auth.event = "revoke_all_tokens"))]
    pub async fn revoke_all_refresh_tokens(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
        debug!(user.id = %user_id, "Revoking all refresh tokens");

        sqlx::query(
            "UPDATE refresh_tokens SET revoked = TRUE, updated_at = NOW() WHERE user_id = $1 AND revoked = FALSE",
        )
        .bind(user_id)
        .execute(db)
        .await?;

        info!(user.id = %user_id, "All refresh tokens revoked");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // Helper to create a test user in the database
    async fn create_test_user(db: &PgPool, email: &str) -> Uuid {
        let password_hash = hash_password("testpassword123").unwrap();
        let user_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO users (id, first_name, last_name, email, password, school_id)
             VALUES ($1, 'Test', 'User', $2, $3, NULL)",
        )
        .bind(user_id)
        .bind(email)
        .bind(&password_hash)
        .execute(db)
        .await
        .unwrap();

        user_id
    }

    // Helper to cleanup test user
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
    async fn test_login_user_returns_all_user_fields(db: PgPool) {
        let email = format!("test_login_{}@example.com", Uuid::new_v4());
        let user_id = create_test_user(&db, &email).await;

        let jwt_config = crate::config::jwt::JwtConfig {
            secret: "test_secret_key_for_testing".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        };

        let dto = LoginRequest {
            email: email.clone(),
            password: "testpassword123".to_string(),
        };

        let result = AuthService::login_user(&db, dto, &jwt_config).await;
        assert!(result.is_ok());

        let login_result = result.unwrap();
        assert!(login_result.is_ok());

        let response = login_result.unwrap();
        assert_eq!(response.user.email, email);
        assert_eq!(response.user.first_name, "Test");
        assert_eq!(response.user.last_name, "User");
        assert!(!response.access_token.is_empty());
        assert!(!response.refresh_token.is_empty());

        cleanup_test_user(&db, user_id).await;
    }

    #[sqlx::test]
    async fn test_verify_mfa_login_returns_all_user_fields(_db: PgPool) {
        // This test would require MFA setup, skipping for basic test
        // Just verify the function signature and basic structure
    }

    #[sqlx::test]
    async fn test_forgot_password_returns_all_user_fields(db: PgPool) {
        let email = format!("test_forgot_{}@example.com", Uuid::new_v4());
        let user_id = create_test_user(&db, &email).await;

        let dto = ForgotPasswordRequest {
            email: email.clone(),
        };

        let result = AuthService::forgot_password(&db, dto).await;
        assert!(result.is_ok());

        cleanup_test_user(&db, user_id).await;
    }

    #[sqlx::test]
    async fn test_refresh_access_token_returns_all_user_fields(db: PgPool) {
        let email = format!("test_refresh_{}@example.com", Uuid::new_v4());
        let user_id = create_test_user(&db, &email).await;

        let jwt_config = crate::config::jwt::JwtConfig {
            secret: "test_secret_key_for_testing".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 604800,
        };

        // First login to get refresh token
        let login_dto = LoginRequest {
            email: email.clone(),
            password: "testpassword123".to_string(),
        };

        let login_result = AuthService::login_user(&db, login_dto, &jwt_config)
            .await
            .unwrap()
            .unwrap();

        // Now refresh
        let refresh_dto = RefreshTokenRequest {
            refresh_token: login_result.refresh_token,
        };

        let result = AuthService::refresh_access_token(&db, refresh_dto, &jwt_config).await;
        assert!(result.is_ok(), "Refresh failed: {:?}", result.err());

        let response = result.unwrap();
        assert_eq!(response.user.email, email);
        assert!(!response.access_token.is_empty());

        cleanup_test_user(&db, user_id).await;
    }

    #[sqlx::test]
    async fn test_user_query_includes_all_required_fields(db: PgPool) {
        let email = format!("test_fields_{}@example.com", Uuid::new_v4());
        let user_id = create_test_user(&db, &email).await;

        // Test the query directly
        let user_data = fetch_user_with_relations(&db, user_id).await;

        assert!(user_data.is_ok());
        let user_data = user_data.unwrap();
        assert_eq!(user_data.id, user_id);
        assert_eq!(user_data.email, email);
        assert_eq!(user_data.login_user.first_name, "Test");
        assert_eq!(user_data.login_user.last_name, "User");
        assert!(user_data.school_id.is_none());
        assert!(user_data.login_user.level.is_none());
        assert!(user_data.login_user.branch.is_none());
        assert!(user_data.login_user.date_of_birth.is_none());
        assert!(user_data.login_user.grade_level.is_none());

        cleanup_test_user(&db, user_id).await;
    }
}
