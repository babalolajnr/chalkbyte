use anyhow::anyhow;
use rayon::prelude::*;
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::instrument;
use uuid::Uuid;

use crate::utils::errors::AppError;
use crate::utils::password::{hash_password, verify_password};

use super::model::{EnableMfaResponse, MfaStatusResponse, RegenerateMfaRecoveryCodesResponse};

pub struct MfaService;

impl MfaService {
    /// Get MFA status for a user
    #[instrument(skip(db))]
    pub async fn get_mfa_status(db: &PgPool, user_id: Uuid) -> Result<MfaStatusResponse, AppError> {
        #[derive(sqlx::FromRow)]
        struct MfaStatus {
            mfa_enabled: bool,
        }

        let status = sqlx::query_as::<_, MfaStatus>("SELECT mfa_enabled FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await?;

        Ok(MfaStatusResponse {
            mfa_enabled: status.mfa_enabled,
        })
    }

    /// Generate MFA secret and QR code for enrollment
    #[instrument(skip(db))]
    pub async fn generate_mfa_secret(
        db: &PgPool,
        user_id: Uuid,
        email: &str,
    ) -> Result<EnableMfaResponse, AppError> {
        // Check if MFA is already enabled
        let status = Self::get_mfa_status(db, user_id).await?;
        if status.mfa_enabled {
            return Err(AppError::bad_request(anyhow!("MFA is already enabled")));
        }

        // Generate new TOTP secret (random 20 bytes base32 encoded)
        // Must be done before any await to maintain Send bound
        let secret_bytes: Vec<u8> = {
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let mut bytes = vec![0u8; 20];
            rng.fill_bytes(&mut bytes);
            bytes
        };
        let secret = Secret::Raw(secret_bytes);
        let secret_encoded = secret.to_encoded().to_string();

        // Create TOTP instance
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.to_bytes().unwrap(),
            Some("Chalkbyte".to_string()),
            email.to_string(),
        )
        .map_err(|e| AppError::internal_error(format!("Failed to create TOTP: {}", e)))?;

        // Generate QR code URL
        let qr_code_url = totp.get_url();

        // Generate QR code as base64 PNG image
        let qr_code_base64 = totp
            .get_qr_base64()
            .map_err(|e| AppError::internal_error(format!("Failed to generate QR code: {}", e)))?;

        // Store the secret temporarily (not enabled yet, waiting for verification)
        sqlx::query("UPDATE users SET mfa_secret = $1 WHERE id = $2")
            .bind(&secret_encoded)
            .bind(user_id)
            .execute(db)
            .await?;

        Ok(EnableMfaResponse {
            secret: secret_encoded.clone(),
            qr_code_url,
            qr_code_base64,
            manual_entry_key: secret_encoded,
        })
    }

    /// Verify TOTP code and enable MFA
    #[instrument(skip(db, code))]
    pub async fn verify_and_enable_mfa(
        db: &PgPool,
        user_id: Uuid,
        code: &str,
    ) -> Result<RegenerateMfaRecoveryCodesResponse, AppError> {
        // Get user's MFA secret and status
        #[derive(sqlx::FromRow)]
        struct UserMfa {
            mfa_enabled: bool,
            mfa_secret: Option<String>,
            email: String,
        }

        let user = sqlx::query_as::<_, UserMfa>(
            "SELECT mfa_enabled, mfa_secret, email FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        if user.mfa_enabled {
            return Err(AppError::bad_request(anyhow!("MFA is already enabled")));
        }

        let secret = user.mfa_secret.ok_or_else(|| {
            AppError::bad_request(anyhow!(
                "MFA secret not initialized. Call /mfa/enable first"
            ))
        })?;

        // Verify TOTP code
        let is_valid = Self::verify_totp(&secret, code, &user.email)?;
        if !is_valid {
            return Err(AppError::bad_request(anyhow!("Invalid TOTP code")));
        }

        // Enable MFA
        sqlx::query("UPDATE users SET mfa_enabled = TRUE WHERE id = $1")
            .bind(user_id)
            .execute(db)
            .await?;

        // Generate and store recovery codes
        let recovery_codes = Self::generate_recovery_codes();
        Self::store_recovery_codes(db, user_id, &recovery_codes).await?;

        Ok(RegenerateMfaRecoveryCodesResponse { recovery_codes })
    }

    /// Verify TOTP code for login
    #[instrument(skip(db, code))]
    pub async fn verify_totp_login(
        db: &PgPool,
        user_id: Uuid,
        code: &str,
    ) -> Result<bool, AppError> {
        #[derive(sqlx::FromRow)]
        struct UserMfa {
            mfa_enabled: bool,
            mfa_secret: Option<String>,
            email: String,
        }

        let user = sqlx::query_as::<_, UserMfa>(
            "SELECT mfa_enabled, mfa_secret, email FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        if !user.mfa_enabled {
            return Err(AppError::bad_request(anyhow!("MFA is not enabled")));
        }

        let secret = user
            .mfa_secret
            .ok_or_else(|| AppError::internal_error("MFA secret not found".to_string()))?;

        Self::verify_totp(&secret, code, &user.email)
    }

    /// Verify recovery code for login
    #[instrument(skip(db, code))]
    pub async fn verify_recovery_code_login(
        db: &PgPool,
        user_id: Uuid,
        code: &str,
    ) -> Result<bool, AppError> {
        Self::verify_recovery_code(db, user_id, code).await
    }

    /// Disable MFA with password confirmation
    #[instrument(skip(db, password))]
    pub async fn disable_mfa(db: &PgPool, user_id: Uuid, password: &str) -> Result<(), AppError> {
        // Verify password
        #[derive(sqlx::FromRow)]
        struct UserPassword {
            password: String,
            mfa_enabled: bool,
        }

        let user = sqlx::query_as::<_, UserPassword>(
            "SELECT password, mfa_enabled FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        if !user.mfa_enabled {
            return Err(AppError::bad_request(anyhow!("MFA is not enabled")));
        }

        let is_valid = verify_password(password, &user.password)?;
        if !is_valid {
            return Err(AppError::bad_request(anyhow!("Invalid password")));
        }

        // Disable MFA and clear secret
        sqlx::query("UPDATE users SET mfa_enabled = FALSE, mfa_secret = NULL WHERE id = $1")
            .bind(user_id)
            .execute(db)
            .await?;

        // Delete recovery codes
        sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await?;

        Ok(())
    }

    /// Regenerate recovery codes
    #[instrument(skip(db))]
    pub async fn regenerate_recovery_codes(
        db: &PgPool,
        user_id: Uuid,
    ) -> Result<RegenerateMfaRecoveryCodesResponse, AppError> {
        // Check if MFA is enabled
        let status = Self::get_mfa_status(db, user_id).await?;
        if !status.mfa_enabled {
            return Err(AppError::bad_request(anyhow!("MFA is not enabled")));
        }

        // Generate new recovery codes
        let recovery_codes = Self::generate_recovery_codes();
        Self::store_recovery_codes(db, user_id, &recovery_codes).await?;

        Ok(RegenerateMfaRecoveryCodesResponse { recovery_codes })
    }

    // Private helper methods

    /// Verify TOTP code
    #[instrument(skip(secret))]
    fn verify_totp(secret: &str, code: &str, email: &str) -> Result<bool, AppError> {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|e| AppError::internal_error(format!("Invalid secret: {}", e)))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some("Chalkbyte".to_string()),
            email.to_string(),
        )
        .map_err(|e| AppError::internal_error(format!("Failed to create TOTP: {}", e)))?;

        Ok(totp
            .check_current(code)
            .map_err(|e| AppError::internal_error(format!("Failed to verify TOTP: {}", e)))?)
    }

    /// Generate recovery codes (10 codes, 8 characters each)
    #[instrument]
    fn generate_recovery_codes() -> Vec<String> {
        use rand::Rng as _;
        let mut rng = rand::thread_rng();
        (0..10)
            .map(|_| {
                (0..8)
                    .map(|_| {
                        let idx = rng.gen_range(0..36);
                        if idx < 10 {
                            (b'0' + idx) as char
                        } else {
                            (b'A' + (idx - 10)) as char
                        }
                    })
                    .collect::<String>()
            })
            .collect()
    }

    /// Store recovery codes in database (hashed)
    #[instrument(skip(db, codes))]
    async fn store_recovery_codes(
        db: &PgPool,
        user_id: Uuid,
        codes: &[String],
    ) -> Result<(), AppError> {
        // Delete existing recovery codes
        sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await?;

        // Hash all codes in parallel using rayon
        let code_hashes: Vec<String> = codes
            .par_iter()
            .map(|code| hash_password(code))
            .collect::<Result<Vec<_>, _>>()?;

        // Batch insert all recovery codes
        sqlx::query(
            r#"
            INSERT INTO mfa_recovery_codes (user_id, code_hash)
            SELECT $1, unnest($2::text[])
            "#,
        )
        .bind(user_id)
        .bind(&code_hashes)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Verify and consume a recovery code
    #[instrument(skip(db, code))]
    async fn verify_recovery_code(
        db: &PgPool,
        user_id: Uuid,
        code: &str,
    ) -> Result<bool, AppError> {
        #[derive(sqlx::FromRow)]
        struct RecoveryCode {
            id: Uuid,
            code_hash: String,
        }

        // Get all unused recovery codes for user
        let codes = sqlx::query_as::<_, RecoveryCode>(
            "SELECT id, code_hash FROM mfa_recovery_codes WHERE user_id = $1 AND used = FALSE",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;

        // Check each code
        for recovery_code in codes {
            if let Ok(valid) = verify_password(code, &recovery_code.code_hash) {
                if valid {
                    // Mark as used
                    sqlx::query(
                        "UPDATE mfa_recovery_codes SET used = TRUE, used_at = NOW() WHERE id = $1",
                    )
                    .bind(recovery_code.id)
                    .execute(db)
                    .await?;

                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
