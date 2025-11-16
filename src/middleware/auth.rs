use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::db::AppState;
use crate::modules::auth::model::Claims;
use crate::utils::errors::AppError;
use crate::utils::jwt::verify_token;

#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing authorization header".to_string()))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::unauthorized("Invalid authorization header format".to_string())
        })?;

        let claims = verify_token(token, &state.jwt_config)?;

        Ok(AuthUser(claims))
    }
}
