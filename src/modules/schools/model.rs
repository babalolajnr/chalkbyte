//! File upload models and validators for the schools module.
//!
//! This module contains DTOs and validation logic for file uploads,
//! particularly for school logos.

use chalkbyte_core::AppError;

/// Metadata about an uploaded file.
///
/// Contains information needed to validate and process a file upload.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// MIME type of the file (e.g., "image/png")
    pub mime_type: String,

    /// Size of the file in bytes
    pub size_bytes: usize,

    /// Original filename (reserved for future use)
    #[allow(dead_code)]
    pub filename: String,
}

/// Validator for school logo uploads.
///
/// Enforces file size and MIME type constraints.
pub struct LogoValidator;

impl LogoValidator {
    /// Allowed MIME types for school logos
    const ALLOWED_MIME_TYPES: &'static [&'static str] = &["image/png", "image/jpeg", "image/webp"];

    /// Maximum logo file size: 5MB
    const MAX_SIZE_BYTES: usize = 5 * 1024 * 1024;

    /// Validate file metadata for logo uploads.
    ///
    /// # Arguments
    /// * `metadata` - File metadata to validate
    ///
    /// # Errors
    /// Returns `AppError::bad_request` if validation fails.
    pub fn validate(metadata: &FileMetadata) -> Result<(), AppError> {
        // Check file size
        if metadata.size_bytes > Self::MAX_SIZE_BYTES {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "File size {} bytes exceeds 5MB limit",
                metadata.size_bytes
            )));
        }

        // Check MIME type
        if !Self::ALLOWED_MIME_TYPES.contains(&metadata.mime_type.as_str()) {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "MIME type '{}' not allowed. Allowed types: PNG, JPEG, WebP",
                metadata.mime_type
            )));
        }

        Ok(())
    }

    /// Get file extension from MIME type.
    ///
    /// # Arguments
    /// * `mime_type` - The MIME type string
    ///
    /// # Returns
    /// File extension without the dot (e.g., "png", "jpg", "webp")
    pub fn get_extension(mime_type: &str) -> &'static str {
        match mime_type {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "bin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_png_logo() {
        let metadata = FileMetadata {
            mime_type: "image/png".to_string(),
            size_bytes: 1024 * 100, // 100KB
            filename: "logo.png".to_string(),
        };

        assert!(LogoValidator::validate(&metadata).is_ok());
    }

    #[test]
    fn test_validate_jpeg_logo() {
        let metadata = FileMetadata {
            mime_type: "image/jpeg".to_string(),
            size_bytes: 1024 * 200, // 200KB
            filename: "logo.jpg".to_string(),
        };

        assert!(LogoValidator::validate(&metadata).is_ok());
    }

    #[test]
    fn test_validate_webp_logo() {
        let metadata = FileMetadata {
            mime_type: "image/webp".to_string(),
            size_bytes: 1024 * 150, // 150KB
            filename: "logo.webp".to_string(),
        };

        assert!(LogoValidator::validate(&metadata).is_ok());
    }

    #[test]
    fn test_validate_rejects_invalid_mime_type() {
        let metadata = FileMetadata {
            mime_type: "text/plain".to_string(),
            size_bytes: 1024,
            filename: "logo.txt".to_string(),
        };

        assert!(LogoValidator::validate(&metadata).is_err());
    }

    #[test]
    fn test_validate_rejects_oversized_file() {
        let metadata = FileMetadata {
            mime_type: "image/png".to_string(),
            size_bytes: 6 * 1024 * 1024, // 6MB
            filename: "logo.png".to_string(),
        };

        assert!(LogoValidator::validate(&metadata).is_err());
    }

    #[test]
    fn test_get_extension_from_mime_type() {
        assert_eq!(LogoValidator::get_extension("image/png"), "png");
        assert_eq!(LogoValidator::get_extension("image/jpeg"), "jpg");
        assert_eq!(LogoValidator::get_extension("image/webp"), "webp");
        assert_eq!(
            LogoValidator::get_extension("application/octet-stream"),
            "bin"
        );
    }
}
