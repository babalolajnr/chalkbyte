//! Student domain models and DTOs.
//!
//! This module contains all data structures related to student management,
//! including student entities, request/response DTOs, and filtering parameters.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Pagination metadata for student responses.
#[derive(Serialize, ToSchema)]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
}

/// Paginated response containing students.
#[derive(Serialize, ToSchema)]
pub struct PaginatedStudentsResponse {
    pub data: Vec<Student>,
    pub meta: PaginationMeta,
}

/// Query parameters for filtering and paginating students.
#[derive(Deserialize, Debug, IntoParams)]
pub struct QueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    /// Required for system admins to specify which school's students to fetch
    pub school_id: Option<Uuid>,
}

impl QueryParams {
    /// Get the page number, defaulting to 1 if not specified.
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Get the limit, defaulting to 10 and clamping between 1 and 100.
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    /// Calculate the offset based on page and limit.
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

/// A student in the system.
///
/// This struct represents the student entity stored in the database.
/// Students are a specialized type of user associated with a school.
#[derive(Serialize, FromRow, Debug, ToSchema)]
pub struct Student {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub school_id: Option<Uuid>,
    #[sqlx(default)]
    pub date_of_birth: Option<chrono::NaiveDate>,
    #[sqlx(default)]
    pub grade_level: Option<String>,
    #[sqlx(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[sqlx(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// DTO for creating a new student.
///
/// Used by admins to create students within their school scope.
#[derive(Deserialize, Debug, ToSchema, Validate)]
pub struct CreateStudentDto {
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,
    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
    #[validate(length(max = 10))]
    pub grade_level: Option<String>,
    /// Required for system admins to specify which school to create the student in
    pub school_id: Option<Uuid>,
}

/// DTO for updating an existing student.
///
/// All fields are optional; only provided fields will be updated.
#[derive(Deserialize, Debug, ToSchema, Validate)]
pub struct UpdateStudentDto {
    #[validate(length(min = 1, max = 100))]
    pub first_name: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub last_name: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 8))]
    pub password: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    #[validate(length(max = 10))]
    pub grade_level: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_params_defaults() {
        let params = QueryParams {
            page: None,
            limit: None,
            school_id: None,
        };
        assert_eq!(params.page(), 1);
        assert_eq!(params.limit(), 10);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_query_params_custom_values() {
        let params = QueryParams {
            page: Some(3),
            limit: Some(25),
            school_id: None,
        };
        assert_eq!(params.page(), 3);
        assert_eq!(params.limit(), 25);
        assert_eq!(params.offset(), 50);
    }

    #[test]
    fn test_query_params_clamping() {
        let params = QueryParams {
            page: Some(-5),
            limit: Some(200),
            school_id: None,
        };
        assert_eq!(params.page(), 1); // Min page is 1
        assert_eq!(params.limit(), 100); // Max limit is 100
    }

    #[test]
    fn test_create_student_dto_validation() {
        let valid_dto = CreateStudentDto {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            password: "password123".to_string(),
            date_of_birth: None,
            grade_level: Some("10".to_string()),
            school_id: None,
        };
        assert!(valid_dto.validate().is_ok());
    }

    #[test]
    fn test_create_student_dto_invalid_email() {
        let invalid_dto = CreateStudentDto {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
            date_of_birth: None,
            grade_level: None,
            school_id: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_create_student_dto_short_password() {
        let invalid_dto = CreateStudentDto {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            password: "short".to_string(),
            date_of_birth: None,
            grade_level: None,
            school_id: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_create_student_dto_empty_name() {
        let invalid_dto = CreateStudentDto {
            first_name: "".to_string(),
            last_name: "Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            password: "password123".to_string(),
            date_of_birth: None,
            grade_level: None,
            school_id: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_create_student_dto_long_name() {
        let invalid_dto = CreateStudentDto {
            first_name: "x".repeat(101),
            last_name: "Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            password: "password123".to_string(),
            date_of_birth: None,
            grade_level: None,
            school_id: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_update_student_dto_validation() {
        let valid_dto = UpdateStudentDto {
            first_name: Some("Jane".to_string()),
            last_name: None,
            email: None,
            password: None,
            date_of_birth: None,
            grade_level: None,
        };
        assert!(valid_dto.validate().is_ok());
    }

    #[test]
    fn test_update_student_dto_empty() {
        let empty_dto = UpdateStudentDto {
            first_name: None,
            last_name: None,
            email: None,
            password: None,
            date_of_birth: None,
            grade_level: None,
        };
        assert!(empty_dto.validate().is_ok());
    }

    #[test]
    fn test_update_student_dto_invalid_email() {
        let invalid_dto = UpdateStudentDto {
            first_name: None,
            last_name: None,
            email: Some("invalid-email".to_string()),
            password: None,
            date_of_birth: None,
            grade_level: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_update_student_dto_short_password() {
        let invalid_dto = UpdateStudentDto {
            first_name: None,
            last_name: None,
            email: None,
            password: Some("short".to_string()),
            date_of_birth: None,
            grade_level: None,
        };
        assert!(invalid_dto.validate().is_err());
    }

    #[test]
    fn test_update_student_dto_long_grade_level() {
        let invalid_dto = UpdateStudentDto {
            first_name: None,
            last_name: None,
            email: None,
            password: None,
            date_of_birth: None,
            grade_level: Some("x".repeat(11)),
        };
        assert!(invalid_dto.validate().is_err());
    }
}
