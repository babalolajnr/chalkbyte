use crate::utils::errors::AppError;
use crate::{db::AppState, validator::ValidatedForm};
use axum::{Json, extract::State};
use tracing::instrument;

use super::{model::RegisterRequestDto, service::AuthService};

#[instrument]
pub async fn register_user(
    State(state): State<AppState>,
    ValidatedForm(dto): ValidatedForm<RegisterRequestDto>,
) -> Result<(), AppError> {
    AuthService::register_user(&state.db, dto).await?;
    Ok(())
}
