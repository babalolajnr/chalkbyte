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
    pub fn validation(err: ValidationErrors) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Validation error: {}", err),
        )
    }

    #[track_caller]
    pub fn form_rejection(err: FormRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Form parsing error: {}", err),
        )
    }

    #[track_caller]
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
