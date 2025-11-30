use anyhow::anyhow;
use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use crate::utils::errors::AppError;

fn format_errors(errors: &ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().filter_map(move |error| {
                error
                    .message
                    .as_ref()
                    .map(|msg| msg.to_string())
                    .or_else(|| Some(format!("{} is invalid", field)))
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|rejection| {
                let error_msg = rejection.body_text();

                if error_msg.contains("missing field") {
                    let field = error_msg
                        .split("missing field `")
                        .nth(1)
                        .and_then(|s| s.split('`').next())
                        .unwrap_or("unknown");
                    return AppError::new(
                        StatusCode::BAD_REQUEST,
                        anyhow!("{} is required", field),
                    );
                }

                if error_msg.contains("invalid type") {
                    return AppError::new(
                        StatusCode::BAD_REQUEST,
                        anyhow!("Invalid field type in request"),
                    );
                }

                if matches!(rejection, JsonRejection::MissingJsonContentType(_)) {
                    return AppError::new(
                        StatusCode::BAD_REQUEST,
                        anyhow!("Missing 'Content-Type: application/json' header"),
                    );
                }

                AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request body"))
            })?;

        value.validate().map_err(|errors| {
            AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                anyhow!("{}", format_errors(&errors)),
            )
        })?;

        Ok(ValidatedJson(value))
    }
}
