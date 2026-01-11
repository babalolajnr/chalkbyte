//! Level domain models and DTOs.
//!
//! This module contains all data structures related to educational levels,
//! including level entities, request/response DTOs, and filtering parameters.

use chalkbyte_core::{PaginationMeta, PaginationParams};
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
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedLevelsResponse {
    pub data: Vec<LevelWithStats>,
    pub meta: PaginationMeta,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_level_dto_validation() {
        let valid_dto = CreateLevelDto {
            name: "Grade 1".to_string(),
            description: Some("First grade".to_string()),
            school_id: None,
        };
        assert!(valid_dto.validate().is_ok());

        let empty_name = CreateLevelDto {
            name: "".to_string(),
            description: None,
            school_id: None,
        };
        assert!(empty_name.validate().is_err());

        let long_name = CreateLevelDto {
            name: "x".repeat(101),
            description: None,
            school_id: None,
        };
        assert!(long_name.validate().is_err());
    }

    #[test]
    fn test_update_level_dto_validation() {
        let valid_dto = UpdateLevelDto {
            name: Some("Updated Grade".to_string()),
            description: Some("Updated description".to_string()),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_update = UpdateLevelDto {
            name: None,
            description: None,
        };
        assert!(empty_update.validate().is_ok());
    }

    #[test]
    fn test_assign_students_dto_validation() {
        let valid_dto = AssignStudentsToLevelDto {
            student_ids: vec![Uuid::new_v4()],
        };
        assert!(valid_dto.validate().is_ok());

        let empty_ids = AssignStudentsToLevelDto {
            student_ids: vec![],
        };
        assert!(empty_ids.validate().is_err());
    }
}
