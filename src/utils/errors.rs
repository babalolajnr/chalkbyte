use anyhow::{Error, anyhow};
use axum::{
    Json,
    extract::rejection::{FormRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub error: Error,
}

impl AppError {
    pub fn new<E>(status: StatusCode, err: E) -> Self
    where
        E: Into<Error>,
    {
        Self {
            status,
            error: err.into(),
        }
    }

    pub fn internal<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    pub fn not_found<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::NOT_FOUND, err)
    }

    pub fn unprocessable<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, err)
    }

    pub fn bad_request<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::BAD_REQUEST, err)
    }

    pub fn database<E>(err: E) -> Self
    where
        E: Into<Error>,
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    pub fn validation(err: ValidationErrors) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Validation error: {}", err),
        )
    }

    pub fn form_rejection(err: FormRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Form parsing error: {}", err),
        )
    }

    pub fn query_rejection(err: QueryRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Query parameter error: {}", err),
        )
    }

    pub fn unauthorized(message: String) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, anyhow!(message))
    }

    pub fn forbidden(message: String) -> Self {
        Self::new(StatusCode::FORBIDDEN, anyhow!(message))
    }

    pub fn internal_error(message: String) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, anyhow!(message))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": self.error.to_string()
        }));

        (self.status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<Error>,
{
    fn from(err: E) -> Self {
        AppError::internal(err)
    }
}
