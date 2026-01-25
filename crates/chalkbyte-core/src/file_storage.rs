//! File storage abstraction layer.
//!
//! This module provides trait-based file storage abstraction to support
//! multiple storage backends (local filesystem, S3, MinIO, etc.) without
//! changing business logic.
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_core::file_storage::{FileStorage, LocalFileStorage};
//! use std::path::PathBuf;
//!
//! let storage = LocalFileStorage::new(
//!     PathBuf::from("./uploads"),
//!     "http://localhost:3000/files".to_string(),
//! );
//!
//! // Save a file
//! let key = storage.save("schools/logo.png", bytes).await?;
//!
//! // Get public URL
//! let url = storage.get_url(&key)?;
//!
//! // Delete a file
//! storage.delete(&key).await?;
//! ```

use std::fmt;
use std::path::PathBuf;
use tokio::fs;

/// Abstract trait for file storage backends.
///
/// Implementations can be swapped without changing business logic.
pub trait FileStorage: Send + Sync {
    /// Save file content and return the storage key.
    ///
    /// # Arguments
    /// * `key` - Unique identifier for the file (e.g., "schools/abc-123.png")
    /// * `content` - File bytes to store
    ///
    /// # Returns
    /// The storage key if successful, or a `StorageError`.
    fn save<'a>(
        &'a self,
        key: &'a str,
        content: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, StorageError>> + Send + 'a>>;

    /// Delete a file by key.
    ///
    /// # Arguments
    /// * `key` - Storage key identifying the file
    ///
    /// # Returns
    /// `Ok(())` if successful or file doesn't exist, or a `StorageError`.
    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), StorageError>> + Send + 'a>>;

    /// Get the public URL for accessing a file.
    ///
    /// # Arguments
    /// * `key` - Storage key identifying the file
    ///
    /// # Returns
    /// The public URL to access the file, or a `StorageError`.
    fn get_url(&self, key: &str) -> Result<String, StorageError>;
}

/// Error type for file storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// File exceeds maximum allowed size.
    InvalidFileSize { max_bytes: usize },

    /// MIME type not allowed.
    InvalidMimeType {
        received: String,
        allowed: Vec<String>,
    },

    /// I/O error (file system or similar).
    IoError(std::io::Error),

    /// File not found.
    NotFound,

    /// Invalid storage key format.
    InvalidKey(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFileSize { max_bytes } => {
                write!(f, "File exceeds maximum size of {} bytes", max_bytes)
            }
            Self::InvalidMimeType { received, allowed } => {
                write!(
                    f,
                    "MIME type '{}' not allowed. Allowed types: {}",
                    received,
                    allowed.join(", ")
                )
            }
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::NotFound => write!(f, "File not found"),
            Self::InvalidKey(msg) => write!(f, "Invalid storage key: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Local filesystem-based file storage implementation.
///
/// Stores files in a directory on the local filesystem and serves them
/// via HTTP URLs.
#[derive(Clone)]
pub struct LocalFileStorage {
    /// Base directory where files are stored
    base_dir: PathBuf,

    /// Base URL for public file access (e.g., "http://localhost:3000/files")
    base_url: String,

    /// Maximum file size in bytes (e.g., 5MB = 5242880)
    max_file_size: usize,

    /// Allowed MIME types (reserved for future use in validation)
    #[allow(dead_code)]
    allowed_mime_types: Vec<String>,
}

impl LocalFileStorage {
    /// Create a new local file storage instance.
    ///
    /// # Arguments
    /// * `base_dir` - Directory to store files
    /// * `base_url` - Public URL prefix for accessing files
    ///
    /// # Example
    /// ```ignore
    /// let storage = LocalFileStorage::new(
    ///     PathBuf::from("./uploads"),
    ///     "http://localhost:3000/files".to_string(),
    /// );
    /// ```
    pub fn new(base_dir: PathBuf, base_url: String) -> Self {
        Self {
            base_dir,
            base_url,
            max_file_size: 5 * 1024 * 1024, // 5MB default
            allowed_mime_types: vec![
                "image/png".to_string(),
                "image/jpeg".to_string(),
                "image/webp".to_string(),
            ],
        }
    }

    /// Create a new local file storage with custom max file size.
    ///
    /// # Arguments
    /// * `base_dir` - Directory to store files
    /// * `base_url` - Public URL prefix for accessing files
    /// * `max_file_size` - Maximum file size in bytes
    pub fn with_max_size(base_dir: PathBuf, base_url: String, max_file_size: usize) -> Self {
        Self {
            base_dir,
            base_url,
            max_file_size,
            allowed_mime_types: vec![
                "image/png".to_string(),
                "image/jpeg".to_string(),
                "image/webp".to_string(),
            ],
        }
    }

    /// Validate storage key format to prevent path traversal.
    fn validate_key(key: &str) -> Result<(), StorageError> {
        // Reject empty keys or keys with path traversal attempts
        if key.is_empty() || key.contains("..") || key.starts_with('/') {
            return Err(StorageError::InvalidKey(
                "Key must not be empty, contain '..', or start with '/'".to_string(),
            ));
        }

        // Allow alphanumeric, hyphens, underscores, slashes, and dots
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
        {
            return Err(StorageError::InvalidKey(
                "Key contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }
}

impl FileStorage for LocalFileStorage {
    fn save<'a>(
        &'a self,
        key: &'a str,
        content: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, StorageError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Validate key
            Self::validate_key(key)?;

            // Check file size
            if content.len() > self.max_file_size {
                return Err(StorageError::InvalidFileSize {
                    max_bytes: self.max_file_size,
                });
            }

            // Construct full path
            let file_path = self.base_dir.join(key);

            // Create parent directories if they don't exist
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write file to disk
            fs::write(&file_path, content).await?;

            // Return the key (storage identifier)
            Ok(key.to_string())
        })
    }

    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), StorageError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Validate key
            Self::validate_key(key)?;

            let file_path = self.base_dir.join(key);

            // Delete file, ignore "not found" errors
            match fs::remove_file(&file_path).await {
                Ok(_) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn get_url(&self, key: &str) -> Result<String, StorageError> {
        // Validate key
        Self::validate_key(key)?;

        // Construct URL by combining base_url with key
        Ok(format!("{}/{}", self.base_url.trim_end_matches('/'), key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key_accepts_valid_keys() {
        assert!(LocalFileStorage::validate_key("schools/logo.png").is_ok());
        assert!(LocalFileStorage::validate_key("schools/abc-123.jpg").is_ok());
        assert!(LocalFileStorage::validate_key("users/profile_pic.webp").is_ok());
    }

    #[test]
    fn test_validate_key_rejects_path_traversal() {
        assert!(LocalFileStorage::validate_key("../../../etc/passwd").is_err());
        assert!(LocalFileStorage::validate_key("..\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_key_rejects_absolute_paths() {
        assert!(LocalFileStorage::validate_key("/etc/passwd").is_err());
        assert!(LocalFileStorage::validate_key("\\windows\\system32").is_err());
    }

    #[test]
    fn test_get_url_formats_correctly() {
        let storage = LocalFileStorage::new(
            PathBuf::from("./uploads"),
            "http://localhost:3000/files".to_string(),
        );

        let url = storage.get_url("schools/logo.png").unwrap();
        assert_eq!(url, "http://localhost:3000/files/schools/logo.png");
    }

    #[test]
    fn test_get_url_handles_trailing_slash() {
        let storage = LocalFileStorage::new(
            PathBuf::from("./uploads"),
            "http://localhost:3000/files/".to_string(),
        );

        let url = storage.get_url("schools/logo.png").unwrap();
        assert_eq!(url, "http://localhost:3000/files/schools/logo.png");
    }
}
