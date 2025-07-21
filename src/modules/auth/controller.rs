use crate::utils::errors::AppError;
use crate::validator::ValidatedJson;
use crate::{db::AppState, validator::ValidatedForm};
use axum::{Json, extract::State};
use tracing::instrument;

use super::{model::RegisterRequestDto, service::AuthService};

#[instrument]
pub async fn register_user(
    State(state): State<AppState>,
    ValidatedJson(dto): ValidatedJson<RegisterRequestDto>,
) -> Result<(), AppError> {
    AuthService::register_user(&state.db, dto).await?;
    Ok(())
}
