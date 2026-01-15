//! Academic Session domain models and DTOs.
//!
//! This module contains all data structures related to academic sessions,
//! including session entities, request/response DTOs, and filtering parameters.
//!
//! Academic sessions represent academic years or periods (e.g., "2025-2026 Academic Year")
//! and serve as containers for terms/semesters. Each school can have multiple sessions,
//! but only one can be active at a time.

use crate::ids::{AcademicSessionId, SchoolId};
use chalkbyte_core::{PaginationMeta, PaginationParams};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Academic session entity representing an academic year/period.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AcademicSession {
    /// Unique identifier for the academic session
    pub id: AcademicSessionId,
    /// Name of the academic session (e.g., "2025-2026 Academic Year")
    pub name: String,
    /// Optional description of the academic session
    pub description: Option<String>,
    /// School this session belongs to
    pub school_id: SchoolId,
    /// Start date of the academic session
    pub start_date: NaiveDate,
    /// End date of the academic session
    pub end_date: NaiveDate,
    /// Whether this session is currently active (only one per school)
    pub is_active: bool,
    /// Timestamp when the session was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the session was last updated
    pub updated_at: DateTime<Utc>,
}

/// Academic session with additional statistics.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AcademicSessionWithStats {
    /// Unique identifier for the academic session
    pub id: AcademicSessionId,
    /// Name of the academic session
    pub name: String,
    /// Optional description of the academic session
    pub description: Option<String>,
    /// School this session belongs to
    pub school_id: SchoolId,
    /// Start date of the academic session
    pub start_date: NaiveDate,
    /// End date of the academic session
    pub end_date: NaiveDate,
    /// Whether this session is currently active
    pub is_active: bool,
    /// Number of terms in this session
    pub term_count: i64,
    /// Timestamp when the session was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the session was last updated
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new academic session.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateAcademicSessionDto {
    /// Name of the academic session (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    /// Optional description of the academic session
    pub description: Option<String>,
    /// School ID - required for system admins, ignored for school admins
    pub school_id: Option<SchoolId>,
    /// Start date of the academic session
    pub start_date: NaiveDate,
    /// End date of the academic session (must be after start_date)
    pub end_date: NaiveDate,
}

/// DTO for updating an existing academic session.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateAcademicSessionDto {
    /// Updated name of the academic session (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    /// Updated description of the academic session
    pub description: Option<String>,
    /// Updated start date of the academic session
    pub start_date: Option<NaiveDate>,
    /// Updated end date of the academic session
    pub end_date: Option<NaiveDate>,
}

/// Query parameters for filtering academic sessions.
#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct AcademicSessionFilterParams {
    /// Filter by school ID (required for system admins)
    pub school_id: Option<SchoolId>,
    /// Filter by active status
    pub is_active: Option<bool>,
    /// Pagination parameters
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Paginated response containing academic sessions.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedAcademicSessionsResponse {
    /// List of academic sessions
    pub data: Vec<AcademicSessionWithStats>,
    /// Pagination metadata
    pub meta: PaginationMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_academic_session_dto_validation() {
        let valid_dto = CreateAcademicSessionDto {
            name: "2025-2026 Academic Year".to_string(),
            description: Some("Main academic year".to_string()),
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_name = CreateAcademicSessionDto {
            name: "".to_string(),
            description: None,
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        };
        assert!(empty_name.validate().is_err());

        let long_name = CreateAcademicSessionDto {
            name: "x".repeat(101),
            description: None,
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        };
        assert!(long_name.validate().is_err());
    }

    #[test]
    fn test_update_academic_session_dto_validation() {
        let valid_dto = UpdateAcademicSessionDto {
            name: Some("Updated Academic Year".to_string()),
            description: Some("Updated description".to_string()),
            start_date: None,
            end_date: None,
        };
        assert!(valid_dto.validate().is_ok());

        let empty_update = UpdateAcademicSessionDto {
            name: None,
            description: None,
            start_date: None,
            end_date: None,
        };
        assert!(empty_update.validate().is_ok());
    }
}
