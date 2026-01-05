use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Level {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub school_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LevelWithStats {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub school_id: Uuid,
    pub student_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateLevelDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    /// School ID - required for system admins, ignored for school admins
    pub school_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateLevelDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct LevelFilterParams {
    pub name: Option<String>,
    /// School ID - required for system admins to scope the query
    pub school_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedLevelsResponse {
    pub data: Vec<LevelWithStats>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AssignStudentsToLevelDto {
    #[validate(length(min = 1))]
    pub student_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MoveStudentToLevelDto {
    pub level_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkAssignResponse {
    pub assigned_count: usize,
    pub failed_ids: Vec<Uuid>,
}
