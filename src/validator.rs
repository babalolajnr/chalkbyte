use anyhow::anyhow;
use axum::{
    Form,
    extract::{FromRequest, Request, rejection::FormRejection},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::utils::errors::AppError;

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
            .map_err(|e| AppError::unprocessable(anyhow!("Validation error: {}", e)))?;

        Ok(ValidatedForm(value))
    }
}
