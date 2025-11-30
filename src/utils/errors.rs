use anyhow::{Error, anyhow};
use axum::{
    Json,
    extract::rejection::{FormRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::backtrace::Backtrace;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub error: Error,
    pub location: Option<&'static std::panic::Location<'static>>,
}

impl AppError {
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

    #[track_caller]
    pub fn internal<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    #[track_caller]
    pub fn not_found<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::NOT_FOUND, err)
    }

    #[track_caller]
    pub fn unprocessable<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, err)
    }

    #[track_caller]
    pub fn bad_request<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::BAD_REQUEST, err)
    }

    #[track_caller]
    pub fn database<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    #[track_caller]
    #[allow(dead_code)]
    pub fn validation(err: ValidationErrors) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Validation error: {}", err),
        )
    }

    #[track_caller]
    #[allow(dead_code)]
    pub fn form_rejection(err: FormRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Form parsing error: {}", err),
        )
    }

    #[track_caller]
    #[allow(dead_code)]
    pub fn query_rejection(err: QueryRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Query parameter error: {}", err),
        )
    }

    #[track_caller]
    pub fn unauthorized(message: String) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, anyhow!(message))
    }

    #[track_caller]
    pub fn forbidden(message: String) -> Self {
        Self::new(StatusCode::FORBIDDEN, anyhow!(message))
    }

    #[track_caller]
    pub fn internal_error(message: String) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, anyhow!(message))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_message = if self.status.is_server_error() {
            let backtrace = Backtrace::capture();
            let backtrace_status = backtrace.status();

            if let Some(location) = self.location {
                error!(
                    status = %self.status.as_u16(),
                    error = %self.error,
                    error_chain = ?self.error.chain().collect::<Vec<_>>(),
                    file = %location.file(),
                    line = %location.line(),
                    column = %location.column(),
                    backtrace_available = ?backtrace_status,
                    "Internal server error occurred"
                );
            } else {
                error!(
                    status = %self.status.as_u16(),
                    error = %self.error,
                    error_chain = ?self.error.chain().collect::<Vec<_>>(),
                    backtrace_available = ?backtrace_status,
                    "Internal server error occurred"
                );
            }

            if backtrace_status == std::backtrace::BacktraceStatus::Captured {
                error!(backtrace = %backtrace, "Error backtrace");
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
}
