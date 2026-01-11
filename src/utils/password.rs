//! Password hashing and verification utilities.
//!
//! This module provides secure password hashing using bcrypt. It wraps the
//! [`bcrypt`] crate to provide a simple API with proper error handling.
//!
//! # Security
//!
//! - Uses bcrypt with the default cost factor (currently 12)
//! - Each password generates a unique salt automatically
//! - Passwords are never stored in plaintext
//!
//! # Example
//!
//! ```ignore
//! use crate::utils::password::{hash_password, verify_password};
//!
//! // Hash a password before storing
//! let hash = hash_password("user_password")?;
//!
//! // Verify a password during login
//! if verify_password("user_password", &hash)? {
//!     println!("Password is correct!");
//! }
//! ```

use bcrypt::{DEFAULT_COST, hash, verify};

use crate::utils::errors::AppError;

/// Hashes a password using bcrypt with the default cost factor.
///
/// This function generates a unique salt for each password, ensuring that
/// identical passwords produce different hashes.
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
///
/// # Returns
///
/// Returns the bcrypt hash string on success, which includes the salt
/// and can be stored directly in the database.
///
/// # Errors
///
/// Returns an [`AppError`] if hashing fails (e.g., due to system errors).
///
/// # Example
///
/// ```ignore
/// let hash = hash_password("secure_password_123")?;
/// // Store `hash` in the database
/// ```
///
/// # Security Note
///
/// The default bcrypt cost factor provides good security but may need
/// adjustment based on your server's performance characteristics.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash(password, DEFAULT_COST)
        .map_err(|e| AppError::internal_error(format!("Failed to hash password: {}", e)))
}

/// Verifies a password against a bcrypt hash.
///
/// This function performs a constant-time comparison to prevent timing attacks.
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The bcrypt hash to verify against (from the database)
///
/// # Returns
///
/// Returns `true` if the password matches the hash, `false` otherwise.
///
/// # Errors
///
/// Returns an [`AppError`] if verification fails due to an invalid hash format.
///
/// # Example
///
/// ```ignore
/// let stored_hash = get_user_password_hash_from_db(user_id)?;
///
/// if verify_password(&submitted_password, &stored_hash)? {
///     // Password is correct, proceed with login
/// } else {
///     // Password is incorrect
/// }
/// ```
///
/// # Security Note
///
/// - Always use this function for password comparison, never compare hashes directly
/// - The bcrypt library handles timing-safe comparison internally
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    verify(password, hash)
        .map_err(|e| AppError::internal_error(format!("Failed to verify password: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_success() {
        let password = "testpassword123";
        let result = hash_password(password);

        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_empty() {
        let password = "";
        let result = hash_password(password);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "correctpassword";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "correctpassword";
        let wrong_password = "wrongpassword";
        let hash = hash_password(password).unwrap();

        let result = verify_password(wrong_password, &hash);

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = "testpassword";
        let invalid_hash = "not_a_valid_bcrypt_hash";

        let result = verify_password(password, invalid_hash);

        assert!(result.is_err());
    }

    #[test]
    fn test_hash_generates_unique_hashes() {
        let password = "samepassword";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        assert_ne!(hash1, hash2);
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_hash_special_characters() {
        let password = "p@ssw0rd!#$%^&*()";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_hash_unicode_characters() {
        let password = "пароль密码🔒";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_hash_long_password() {
        let password = "a".repeat(100);
        let hash = hash_password(&password).unwrap();

        let result = verify_password(&password, &hash);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_case_sensitive() {
        let password = "Password123";
        let hash = hash_password(password).unwrap();

        let result1 = verify_password("password123", &hash);
        let result2 = verify_password("PASSWORD123", &hash);

        assert!(result1.is_ok());
        assert!(!result1.unwrap());
        assert!(result2.is_ok());
        assert!(!result2.unwrap());
    }
}
