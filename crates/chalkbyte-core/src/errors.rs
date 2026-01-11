//! Application error handling utilities.
//!
//! This module provides a unified error type [`AppError`] for handling errors
//! throughout the application. It wraps various error sources and converts them
//! into appropriate HTTP responses.
//!
//! # Example
//!
//! ```ignore
//! use crate::utils::errors::AppError;
//!
//! fn example_handler() -> Result<(), AppError> {
//!     // Return a 404 error
//!     Err(AppError::not_found(anyhow::anyhow!("Resource not found")))
//! }
//! ```

use anyhow::{Error, anyhow};
use axum::{
    Json,
    extract::rejection::{FormRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::error;
use validator::ValidationErrors;

/// Application-wide error type that converts into HTTP responses.
///
/// `AppError` wraps an [`anyhow::Error`] along with an HTTP status code and
/// optional source location for debugging. It implements [`IntoResponse`] to
/// automatically convert into appropriate HTTP responses.
///
/// # Error Handling Strategy
///
/// - **Client errors (4xx)**: The error message is returned to the client
/// - **Server errors (5xx)**: A generic message is returned, and the actual
///   error is logged with source location
///
/// # Example
///
/// ```ignore
/// use crate::utils::errors::AppError;
/// use axum::http::StatusCode;
///
/// // Using convenience constructors
/// let not_found = AppError::not_found(anyhow::anyhow!("User not found"));
/// let bad_request = AppError::bad_request(anyhow::anyhow!("Invalid input"));
///
/// // Using the generic constructor
/// let custom = AppError::new(StatusCode::IM_A_TEAPOT, anyhow::anyhow!("I'm a teapot"));
/// ```
#[derive(Debug)]
pub struct AppError {
    /// HTTP status code to return
    pub status: StatusCode,
    /// The underlying error
    pub error: Error,
    /// Source location where the error was created (for debugging)
    pub location: Option<&'static std::panic::Location<'static>>,
}

impl AppError {
    /// Creates a new `AppError` with the specified status code and error.
    ///
    /// The source location is automatically captured using `#[track_caller]`.
    ///
    /// # Arguments
    ///
    /// * `status` - HTTP status code for the response
    /// * `err` - Any error type that can be converted into [`anyhow::Error`]
    ///
    /// # Example
    ///
    /// ```ignore
    /// use axum::http::StatusCode;
    /// use crate::utils::errors::AppError;
    ///
    /// let error = AppError::new(StatusCode::CONFLICT, anyhow::anyhow!("Resource conflict"));
    /// ```
    #[track_caller]
    pub fn new<E>(status: StatusCode, err: E) -> Self
    where
        E: Into<Error>,
    {
        Self {
            status,
            error: err.into(),
            location: Some(std::panic::Location::caller()),
        }
    }

    /// Creates an internal server error (500).
    ///
    /// Use this for unexpected errors that indicate a bug or system failure.
    /// The actual error message will be logged but not exposed to clients.
    #[track_caller]
    pub fn internal<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    /// Creates a not found error (404).
    ///
    /// Use this when a requested resource does not exist.
    #[track_caller]
    pub fn not_found<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::NOT_FOUND, err)
    }

    /// Creates an unprocessable entity error (422).
    ///
    /// Use this when the request is syntactically correct but semantically invalid,
    /// such as validation failures or business rule violations.
    #[track_caller]
    pub fn unprocessable<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, err)
    }

    /// Creates a bad request error (400).
    ///
    /// Use this when the client sends a malformed or invalid request.
    #[track_caller]
    pub fn bad_request<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::BAD_REQUEST, err)
    }

    /// Creates a database error (500 Internal Server Error).
    ///
    /// Use this for database-related errors. The actual error is logged
    /// but a generic message is returned to clients.
    #[track_caller]
    pub fn database<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    /// Creates a validation error (400) from [`ValidationErrors`].
    ///
    /// Use this when request validation fails.
    #[track_caller]
    #[allow(dead_code)]
    pub fn validation(err: ValidationErrors) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Validation error: {}", err),
        )
    }

    /// Creates a bad request error (400) from a form rejection.
    ///
    /// Use this when form parsing fails in Axum extractors.
    #[track_caller]
    #[allow(dead_code)]
    pub fn form_rejection(err: FormRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Form parsing error: {}", err),
        )
    }

    /// Creates a bad request error (400) from a query rejection.
    ///
    /// Use this when query parameter parsing fails in Axum extractors.
    #[track_caller]
    pub fn query_rejection(err: QueryRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Query parameter error: {}", err),
        )
    }

    /// Creates an unauthorized error (401).
    ///
    /// Use this when authentication fails or is missing.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing why authentication failed
    #[track_caller]
    pub fn unauthorized(message: String) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, anyhow!(message))
    }

    /// Creates a forbidden error (403).
    ///
    /// Use this when the user is authenticated but lacks permission
    /// to access the requested resource.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing why access was denied
    #[track_caller]
    pub fn forbidden(message: String) -> Self {
        Self::new(StatusCode::FORBIDDEN, anyhow!(message))
    }

    /// Creates an internal server error (500) with a custom message.
    ///
    /// Use this for unexpected errors with a specific error message.
    /// The message will be logged but not exposed to clients.
    #[track_caller]
    pub fn internal_error(message: String) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, anyhow!(message))
    }

    /// Returns `true` if this is a server error (5xx status code).
    #[must_use]
    #[allow(dead_code)]
    pub fn is_server_error(&self) -> bool {
        self.status.is_server_error()
    }

    /// Returns `true` if this is a client error (4xx status code).
    #[must_use]
    #[allow(dead_code)]
    pub fn is_client_error(&self) -> bool {
        self.status.is_client_error()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_message = if self.status.is_server_error() {
            if let Some(location) = self.location {
                error!(
                    status = %self.status.as_u16(),
                    error = %self.error,
                    file = %location.file(),
                    line = %location.line(),
                    "Internal server error"
                );
            } else {
                error!(
                    status = %self.status.as_u16(),
                    error = %self.error,
                    "Internal server error"
                );
            }

            "Internal server error".to_string()
        } else {
            self.error.to_string()
        };

        let body = Json(json!({
            "error": error_message
        }));

        (self.status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<Error>,
{
    #[track_caller]
    fn from(err: E) -> Self {
        AppError::internal(err)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.status.as_u16(), self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_app_error_new() {
        let error = AppError::new(StatusCode::BAD_REQUEST, anyhow!("Test error"));
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_internal() {
        let error = AppError::internal(anyhow!("Internal error"));
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_not_found() {
        let error = AppError::not_found(anyhow!("Not found"));
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_unprocessable() {
        let error = AppError::unprocessable(anyhow!("Unprocessable entity"));
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_bad_request() {
        let error = AppError::bad_request(anyhow!("Bad request"));
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_database() {
        let error = AppError::database(anyhow!("Database error"));
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_unauthorized() {
        let error = AppError::unauthorized("Unauthorized access".to_string());
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
        assert_eq!(error.error.to_string(), "Unauthorized access");
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_forbidden() {
        let error = AppError::forbidden("Forbidden access".to_string());
        assert_eq!(error.status, StatusCode::FORBIDDEN);
        assert_eq!(error.error.to_string(), "Forbidden access");
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_internal_error() {
        let error = AppError::internal_error("Internal server error".to_string());
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error.to_string(), "Internal server error");
        assert!(error.location.is_some());
    }

    #[test]
    fn test_app_error_from_anyhow() {
        let anyhow_error = anyhow!("Test anyhow error");
        let app_error: AppError = anyhow_error.into();
        assert_eq!(app_error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(app_error.location.is_some());
    }

    #[test]
    fn test_app_error_from_string() {
        let string_error = "String error".to_string();
        let app_error: AppError = anyhow!(string_error).into();
        assert_eq!(app_error.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_error_messages_preserved() {
        let custom_message = "Custom error message with special chars: !@#$%";
        let error = AppError::bad_request(anyhow!(custom_message));
        assert_eq!(error.error.to_string(), custom_message);
    }

    #[test]
    fn test_app_error_different_status_codes() {
        let statuses = vec![
            (
                StatusCode::BAD_REQUEST,
                AppError::bad_request(anyhow!("bad")),
            ),
            (
                StatusCode::NOT_FOUND,
                AppError::not_found(anyhow!("not found")),
            ),
            (
                StatusCode::FORBIDDEN,
                AppError::forbidden("forbidden".to_string()),
            ),
            (
                StatusCode::UNAUTHORIZED,
                AppError::unauthorized("unauthorized".to_string()),
            ),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                AppError::unprocessable(anyhow!("unprocessable")),
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::internal(anyhow!("internal")),
            ),
        ];

        for (expected_status, error) in statuses {
            assert_eq!(error.status, expected_status);
        }
    }

    #[test]
    fn test_app_error_location_tracking() {
        let error1 = AppError::bad_request(anyhow!("error1"));
        let error2 = AppError::not_found(anyhow!("error2"));

        assert!(error1.location.is_some());
        assert!(error2.location.is_some());

        let loc1 = error1.location.unwrap();
        let loc2 = error2.location.unwrap();

        assert_ne!(loc1.line(), loc2.line());
    }

    #[test]
    fn test_app_error_chain() {
        let base_error = anyhow!("Base error");
        let wrapped_error = base_error.context("Wrapped context");
        let app_error = AppError::internal(wrapped_error);

        assert_eq!(app_error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(app_error.error.to_string().contains("Wrapped context"));
    }

    #[test]
    fn test_app_error_validation() {
        use validator::ValidationErrors;
        let errors = ValidationErrors::new();
        let error = AppError::validation(errors);
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_is_server_error() {
        let server_error = AppError::internal(anyhow!("server error"));
        let client_error = AppError::bad_request(anyhow!("client error"));

        assert!(server_error.is_server_error());
        assert!(!client_error.is_server_error());
    }

    #[test]
    fn test_is_client_error() {
        let server_error = AppError::internal(anyhow!("server error"));
        let client_error = AppError::bad_request(anyhow!("client error"));

        assert!(!server_error.is_client_error());
        assert!(client_error.is_client_error());
    }

    #[test]
    fn test_display_impl() {
        let error = AppError::not_found(anyhow!("Resource not found"));
        let display = format!("{}", error);
        assert!(display.contains("404"));
        assert!(display.contains("Resource not found"));
    }
}
