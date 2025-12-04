use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Branch {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub level_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BranchWithStats {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub level_id: Uuid,
    pub student_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateBranchDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateBranchDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct BranchFilterParams {
    pub name: Option<String>,
    #[serde(flatten)]
    pub pagination: crate::utils::pagination::PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedBranchesResponse {
    pub data: Vec<BranchWithStats>,
    pub meta: crate::utils::pagination::PaginationMeta,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AssignStudentsToBranchDto {
    #[validate(length(min = 1))]
    pub student_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MoveStudentToBranchDto {
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkAssignResponse {
    pub assigned_count: usize,
    pub failed_ids: Vec<Uuid>,
}
