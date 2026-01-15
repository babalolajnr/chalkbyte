//! Term domain models and DTOs.
//!
//! This module contains all data structures related to terms/semesters,
//! including term entities, request/response DTOs, and filtering parameters.
//!
//! Terms represent subdivisions within an academic session (e.g., "Fall Semester",
//! "Term 1", "Q1"). Each academic session can have multiple terms, and schools
//! can define their own term structure (semesters, trimesters, quarters, etc.).

use crate::ids::{AcademicSessionId, SchoolId, TermId};
use chalkbyte_core::{PaginationMeta, PaginationParams};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Term entity representing a period within an academic session.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Term {
    /// Unique identifier for the term
    pub id: TermId,
    /// Name of the term (e.g., "Fall Semester", "Term 1")
    pub name: String,
    /// Optional description of the term
    pub description: Option<String>,
    /// Academic session this term belongs to
    pub academic_session_id: AcademicSessionId,
    /// Start date of the term
    pub start_date: NaiveDate,
    /// End date of the term
    pub end_date: NaiveDate,
    /// Order/sequence of the term within the session (1, 2, 3, etc.)
    pub sequence: i32,
    /// Whether this term is the current active term
    pub is_current: bool,
    /// Timestamp when the term was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the term was last updated
    pub updated_at: DateTime<Utc>,
}

/// Term with additional session and school information.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TermWithSessionInfo {
    /// Unique identifier for the term
    pub id: TermId,
    /// Name of the term
    pub name: String,
    /// Optional description of the term
    pub description: Option<String>,
    /// Academic session this term belongs to
    pub academic_session_id: AcademicSessionId,
    /// Start date of the term
    pub start_date: NaiveDate,
    /// End date of the term
    pub end_date: NaiveDate,
    /// Order/sequence of the term within the session
    pub sequence: i32,
    /// Whether this term is the current active term
    pub is_current: bool,
    /// Name of the parent academic session
    pub session_name: String,
    /// School ID (from the parent session)
    pub school_id: SchoolId,
    /// Whether the parent session is active
    pub session_is_active: bool,
    /// Timestamp when the term was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the term was last updated
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new term.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateTermDto {
    /// Name of the term (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    /// Optional description of the term
    pub description: Option<String>,
    /// Academic session ID - can be provided in URL path instead
    pub academic_session_id: Option<AcademicSessionId>,
    /// Start date of the term
    pub start_date: NaiveDate,
    /// End date of the term (must be after start_date)
    pub end_date: NaiveDate,
    /// Order/sequence of the term within the session (optional, auto-calculated if not provided)
    #[validate(range(min = 1))]
    pub sequence: Option<i32>,
}

/// DTO for updating an existing term.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateTermDto {
    /// Updated name of the term (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    /// Updated description of the term
    pub description: Option<String>,
    /// Updated start date of the term
    pub start_date: Option<NaiveDate>,
    /// Updated end date of the term
    pub end_date: Option<NaiveDate>,
    /// Updated sequence of the term within the session
    #[validate(range(min = 1))]
    pub sequence: Option<i32>,
}

/// Query parameters for filtering terms.
#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct TermFilterParams {
    /// Filter by academic session ID
    pub academic_session_id: Option<AcademicSessionId>,
    /// Filter by current status
    pub is_current: Option<bool>,
    /// Filter by school ID (for getting current term across all sessions)
    pub school_id: Option<SchoolId>,
    /// Pagination parameters
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Paginated response containing terms.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedTermsResponse {
    /// List of terms
    pub data: Vec<Term>,
    /// Pagination metadata
    pub meta: PaginationMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_term_dto_validation() {
        let valid_dto = CreateTermDto {
            name: "Fall Semester".to_string(),
            description: Some("First semester of the year".to_string()),
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            sequence: Some(1),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_name = CreateTermDto {
            name: "".to_string(),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            sequence: None,
        };
        assert!(empty_name.validate().is_err());

        let long_name = CreateTermDto {
            name: "x".repeat(101),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            sequence: None,
        };
        assert!(long_name.validate().is_err());

        let invalid_sequence = CreateTermDto {
            name: "Term".to_string(),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            sequence: Some(0),
        };
        assert!(invalid_sequence.validate().is_err());
    }

    #[test]
    fn test_update_term_dto_validation() {
        let valid_dto = UpdateTermDto {
            name: Some("Spring Semester".to_string()),
            description: Some("Updated description".to_string()),
            start_date: None,
            end_date: None,
            sequence: Some(2),
        };
        assert!(valid_dto.validate().is_ok());

        let empty_update = UpdateTermDto {
            name: None,
            description: None,
            start_date: None,
            end_date: None,
            sequence: None,
        };
        assert!(empty_update.validate().is_ok());

        let invalid_sequence = UpdateTermDto {
            name: None,
            description: None,
            start_date: None,
            end_date: None,
            sequence: Some(0),
        };
        assert!(invalid_sequence.validate().is_err());
    }
}
