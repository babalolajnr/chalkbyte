use anyhow::anyhow;
use axum::{
    Form, Json,
    extract::{
        FromRequest, Request,
        rejection::{FormRejection, JsonRejection},
    },
    http::header::CONTENT_TYPE,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use crate::utils::errors::AppError;

/// Formats validation errors into a human-readable string.
///
/// Iterates over each field and its associated errors, generating a message
/// for each error. If a custom message is provided, it is used; otherwise,
/// a default message is generated based on the error code and parameters.
///
/// # Arguments
///
/// * `errors` - A reference to `ValidationErrors` containing field errors.
///
/// # Returns
///
/// A comma-separated string of formatted error messages.
fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut messages = Vec::new();

    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            let message = error
                .message
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| match error.code.as_ref() {
                    "email" => format!("{} must be a valid email address", field),
                    "length" => {
                        if let Some(min) = error.params.get("min") {
                            format!("{} must be at least {} characters long", field, min)
                        } else if let Some(max) = error.params.get("max") {
                            format!("{} must be at most {} characters long", field, max)
                        } else {
                            format!("{} has invalid length", field)
                        }
                    }
                    "range" => format!("{} is out of range", field),
                    "required" => format!("{} is required", field),
                    _ => format!("{} is invalid", field),
                });
            messages.push(message);
        }
    }

    messages.join(", ")
}

#[derive(Debug, Clone, Default, Copy)]
pub struct ValidatedForm<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::bad_request(anyhow!("Failed to parse form: {}", e)))?;

        value
            .validate()
            .map_err(|e| AppError::bad_request(anyhow!("{}", format_validation_errors(&e))))?;

        Ok(ValidatedForm(value))
    }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::bad_request(anyhow!("Failed to parse JSON: {}", e)))?;

        value
            .validate()
            .map_err(|e| AppError::unprocessable(anyhow!("{}", format_validation_errors(&e))))?;

        Ok(ValidatedJson(value))
    }
}

// #[derive(Debug, Clone, Default, Copy)]
pub struct Validated<T>(pub T);

// Generic extractor that handles both forms and JSON based on Content-Type
impl<T, S> FromRequest<S> for Validated<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");

        let value = if content_type.starts_with("application/json") {
            // Parse as JSON
            let Json(value) = Json::<T>::from_request(req, state)
                .await
                .map_err(|e| AppError::bad_request(anyhow!("Failed to parse JSON: {}", e)))?;
            value
        } else if content_type.starts_with("application/x-www-form-urlencoded")
            || content_type.starts_with("multipart/form-data")
        {
            // Parse as Form
            let Form(value) = Form::<T>::from_request(req, state)
                .await
                .map_err(|e| AppError::bad_request(anyhow!("Failed to parse form: {}", e)))?;
            value
        } else {
            // Default to JSON if content type is not specified or unrecognized
            let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
                AppError::bad_request(anyhow!("Failed to parse JSON (default): {}", e))
            })?;
            value
        };

        value
            .validate()
            .map_err(|e| AppError::unprocessable(anyhow!("{}", format_validation_errors(&e))))?;

        Ok(Validated(value))
    }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct ValidatedAuto<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedAuto<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Try JSON first
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts.clone(), body);

        let value = match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => value,
            Err(_) => {
                // If JSON fails, try Form
                let req = Request::from_parts(parts, axum::body::Body::empty());
                let Form(value) = Form::<T>::from_request(req, state).await.map_err(|e| {
                    AppError::bad_request(anyhow!("Failed to parse as JSON or form: {}", e))
                })?;
                value
            }
        };

        value
            .validate()
            .map_err(|e| AppError::unprocessable(anyhow!("{}", format_validation_errors(&e))))?;

        Ok(ValidatedAuto(value))
    }
}
