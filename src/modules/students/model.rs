use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, ToSchema)]
pub struct PaginatedStudentsResponse {
    pub data: Vec<Student>,
    pub meta: PaginationMeta,
}

#[derive(Serialize, ToSchema)]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
}

#[derive(Deserialize, Debug, IntoParams)]
pub struct QueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl QueryParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).max(1).min(100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

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
}

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
