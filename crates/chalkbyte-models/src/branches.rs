//! Branch domain models and DTOs.
//!
//! This module contains all data structures related to school branches,
//! including branch entities, request/response DTOs, and filtering parameters.

use crate::ids::{BranchId, LevelId, UserId};
use chalkbyte_core::{PaginationMeta, PaginationParams};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    pub description: Option<String>,
    pub level_id: LevelId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BranchWithStats {
    pub id: BranchId,
    pub name: String,
    pub description: Option<String>,
    pub level_id: LevelId,
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
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedBranchesResponse {
    pub data: Vec<BranchWithStats>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AssignStudentsToBranchDto {
    #[validate(length(min = 1))]
    pub student_ids: Vec<UserId>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MoveStudentToBranchDto {
    pub branch_id: Option<BranchId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkAssignResponse {
    pub assigned_count: usize,
    pub failed_ids: Vec<UserId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_branch_dto_validation() {
        let valid_dto = CreateBranchDto {
            name: "Science Branch".to_string(),
            description: Some("Science focused branch".to_string()),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_name = CreateBranchDto {
            name: "".to_string(),
            description: None,
        };
        assert!(empty_name.validate().is_err());

        let long_name = CreateBranchDto {
            name: "x".repeat(101),
            description: None,
        };
        assert!(long_name.validate().is_err());
    }

    #[test]
    fn test_update_branch_dto_validation() {
        let valid_dto = UpdateBranchDto {
            name: Some("Updated Branch".to_string()),
            description: Some("Updated description".to_string()),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_update = UpdateBranchDto {
            name: None,
            description: None,
        };
        assert!(empty_update.validate().is_ok());
    }

    #[test]
    fn test_assign_students_dto_validation() {
        let valid_dto = AssignStudentsToBranchDto {
            student_ids: vec![UserId::new()],
        };
        assert!(valid_dto.validate().is_ok());

        let empty_ids = AssignStudentsToBranchDto {
            student_ids: vec![],
        };
        assert!(empty_ids.validate().is_err());
    }
}
