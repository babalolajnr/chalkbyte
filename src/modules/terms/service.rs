use chrono::NaiveDate;
use sqlx::PgPool;
use tracing::instrument;

use chalkbyte_core::{AppError, PaginationMeta};
use chalkbyte_models::ids::{AcademicSessionId, SchoolId, TermId};

use crate::modules::academic_sessions::model::AcademicSession;
use crate::modules::terms::model::{
    CreateTermDto, PaginatedTermsResponse, Term, TermFilterParams, TermWithSessionInfo,
    UpdateTermDto,
};

pub struct TermService;

impl TermService {
    /// Check if two date ranges overlap.
    fn dates_overlap(
        start1: NaiveDate,
        end1: NaiveDate,
        start2: NaiveDate,
        end2: NaiveDate,
    ) -> bool {
        start1 < end2 && start2 < end1
    }

    /// Validate term dates are within session range and don't overlap with existing terms.
    async fn validate_term_dates(
        db: &PgPool,
        session: &AcademicSession,
        start_date: NaiveDate,
        end_date: NaiveDate,
        exclude_term_id: Option<TermId>,
    ) -> Result<(), AppError> {
        // Validate dates are in correct order
        if start_date >= end_date {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Start date must be before end date"
            )));
        }

        // Validate term dates are within session range
        if start_date < session.start_date || end_date > session.end_date {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Term dates must be within the academic session date range ({} to {})",
                session.start_date,
                session.end_date
            )));
        }

        // Check for overlapping terms
        let existing_terms = sqlx::query_as::<_, Term>(
            r#"SELECT id, name, description, academic_session_id, start_date, end_date, sequence, is_current, created_at, updated_at
               FROM terms WHERE academic_session_id = $1"#,
        )
        .bind(session.id)
        .fetch_all(db)
        .await?;

        for term in existing_terms {
            // Skip the term being updated
            if let Some(exclude_id) = exclude_term_id
                && term.id == exclude_id
            {
                continue;
            }

            if Self::dates_overlap(start_date, end_date, term.start_date, term.end_date) {
                return Err(AppError::bad_request(anyhow::anyhow!(
                    "Term dates overlap with existing term: {} ({} to {})",
                    term.name,
                    term.start_date,
                    term.end_date
                )));
            }
        }

        Ok(())
    }

    /// Create a new term within an academic session.
    #[instrument(skip(db))]
    pub async fn create_term(
        db: &PgPool,
        session_id: AcademicSessionId,
        dto: CreateTermDto,
    ) -> Result<Term, AppError> {
        // Get the session to validate dates
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"SELECT id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at
               FROM academic_sessions WHERE id = $1"#,
        )
        .bind(session_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        // Validate dates
        Self::validate_term_dates(db, &session, dto.start_date, dto.end_date, None).await?;

        // Get next sequence number if not provided
        let sequence = if let Some(seq) = dto.sequence {
            seq
        } else {
            let max_seq = sqlx::query_scalar::<_, Option<i32>>(
                "SELECT MAX(sequence) FROM terms WHERE academic_session_id = $1",
            )
            .bind(session_id)
            .fetch_one(db)
            .await?;
            max_seq.unwrap_or(0) + 1
        };

        let term = sqlx::query_as::<_, Term>(
            r#"INSERT INTO terms (name, description, academic_session_id, start_date, end_date, sequence)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, name, description, academic_session_id, start_date, end_date, sequence, is_current, created_at, updated_at"#,
        )
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(session_id)
        .bind(dto.start_date)
        .bind(dto.end_date)
        .bind(sequence)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                if db_err.message().contains("unique_term_name_per_session") {
                    return AppError::bad_request(anyhow::anyhow!(
                        "A term with this name already exists in this session"
                    ));
                }
                if db_err.message().contains("unique_term_sequence_per_session") {
                    return AppError::bad_request(anyhow::anyhow!(
                        "A term with this sequence already exists in this session"
                    ));
                }
            }
            AppError::from(e)
        })?;

        Ok(term)
    }

    /// Get paginated list of terms for an academic session.
    #[instrument(skip(db))]
    pub async fn get_terms_by_session(
        db: &PgPool,
        session_id: AcademicSessionId,
        filters: TermFilterParams,
    ) -> Result<PaginatedTermsResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        // Verify session exists
        let session_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM academic_sessions WHERE id = $1)",
        )
        .bind(session_id)
        .fetch_one(db)
        .await?;

        if !session_exists {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Academic session not found"
            )));
        }

        let mut count_query =
            String::from("SELECT COUNT(*) FROM terms WHERE academic_session_id = $1");
        let mut where_clause = String::new();

        if let Some(is_current) = filters.is_current {
            where_clause.push_str(&format!(" AND is_current = {}", is_current));
        }

        count_query.push_str(&where_clause);

        let total = sqlx::query_scalar::<_, i64>(&count_query)
            .bind(session_id)
            .fetch_one(db)
            .await?;

        let mut data_query = String::from(
            r#"SELECT id, name, description, academic_session_id, start_date, end_date, sequence, is_current, created_at, updated_at
               FROM terms WHERE academic_session_id = $1"#,
        );
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY sequence ASC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let terms = sqlx::query_as::<_, Term>(&data_query)
            .bind(session_id)
            .fetch_all(db)
            .await?;

        let has_more = offset + limit < total;

        Ok(PaginatedTermsResponse {
            data: terms,
            meta: PaginationMeta {
                total,
                limit,
                offset: Some(offset),
                page: None,
                has_more,
            },
        })
    }

    /// Get a term by ID with session info.
    #[instrument(skip(db))]
    pub async fn get_term_by_id(
        db: &PgPool,
        term_id: TermId,
    ) -> Result<TermWithSessionInfo, AppError> {
        let term = sqlx::query_as::<_, TermWithSessionInfo>(
            r#"SELECT
                t.id,
                t.name,
                t.description,
                t.academic_session_id,
                t.start_date,
                t.end_date,
                t.sequence,
                t.is_current,
                t.created_at,
                t.updated_at,
                s.name as session_name,
                s.school_id,
                s.is_active as session_is_active
               FROM terms t
               JOIN academic_sessions s ON s.id = t.academic_session_id
               WHERE t.id = $1"#,
        )
        .bind(term_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Term not found")))?;

        Ok(term)
    }

    /// Get a term by ID with school filtering.
    #[instrument(skip(db))]
    pub async fn get_term_by_id_with_school_filter(
        db: &PgPool,
        term_id: TermId,
        school_id: SchoolId,
    ) -> Result<TermWithSessionInfo, AppError> {
        let term = sqlx::query_as::<_, TermWithSessionInfo>(
            r#"SELECT
                t.id,
                t.name,
                t.description,
                t.academic_session_id,
                t.start_date,
                t.end_date,
                t.sequence,
                t.is_current,
                t.created_at,
                t.updated_at,
                s.name as session_name,
                s.school_id,
                s.is_active as session_is_active
               FROM terms t
               JOIN academic_sessions s ON s.id = t.academic_session_id
               WHERE t.id = $1 AND s.school_id = $2"#,
        )
        .bind(term_id)
        .bind(school_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Term not found")))?;

        Ok(term)
    }

    /// Get the current term for a school's active session.
    #[instrument(skip(db))]
    pub async fn get_current_term(
        db: &PgPool,
        school_id: SchoolId,
    ) -> Result<Option<TermWithSessionInfo>, AppError> {
        let term = sqlx::query_as::<_, TermWithSessionInfo>(
            r#"SELECT
                t.id,
                t.name,
                t.description,
                t.academic_session_id,
                t.start_date,
                t.end_date,
                t.sequence,
                t.is_current,
                t.created_at,
                t.updated_at,
                s.name as session_name,
                s.school_id,
                s.is_active as session_is_active
               FROM terms t
               JOIN academic_sessions s ON s.id = t.academic_session_id
               WHERE s.school_id = $1 AND s.is_active = TRUE AND t.is_current = TRUE"#,
        )
        .bind(school_id)
        .fetch_optional(db)
        .await?;

        Ok(term)
    }

    /// Update a term.
    #[instrument(skip(db))]
    pub async fn update_term(
        db: &PgPool,
        term_id: TermId,
        dto: UpdateTermDto,
    ) -> Result<Term, AppError> {
        // Get existing term with session info
        let existing = sqlx::query_as::<_, TermWithSessionInfo>(
            r#"SELECT
                t.id,
                t.name,
                t.description,
                t.academic_session_id,
                t.start_date,
                t.end_date,
                t.sequence,
                t.is_current,
                t.created_at,
                t.updated_at,
                s.name as session_name,
                s.school_id,
                s.is_active as session_is_active
               FROM terms t
               JOIN academic_sessions s ON s.id = t.academic_session_id
               WHERE t.id = $1"#,
        )
        .bind(term_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Term not found")))?;

        let name = dto.name.unwrap_or(existing.name);
        let description = if dto.description.is_some() {
            dto.description
        } else {
            existing.description
        };
        let start_date = dto.start_date.unwrap_or(existing.start_date);
        let end_date = dto.end_date.unwrap_or(existing.end_date);
        let sequence = dto.sequence.unwrap_or(existing.sequence);

        // Get session for date validation
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"SELECT id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at
               FROM academic_sessions WHERE id = $1"#,
        )
        .bind(existing.academic_session_id)
        .fetch_one(db)
        .await?;

        // Validate dates (excluding this term from overlap check)
        Self::validate_term_dates(db, &session, start_date, end_date, Some(term_id)).await?;

        let term = sqlx::query_as::<_, Term>(
            r#"UPDATE terms
               SET name = $1, description = $2, start_date = $3, end_date = $4, sequence = $5, updated_at = NOW()
               WHERE id = $6
               RETURNING id, name, description, academic_session_id, start_date, end_date, sequence, is_current, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&description)
        .bind(start_date)
        .bind(end_date)
        .bind(sequence)
        .bind(term_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                if db_err.message().contains("unique_term_name_per_session") {
                    return AppError::bad_request(anyhow::anyhow!(
                        "A term with this name already exists in this session"
                    ));
                }
                if db_err.message().contains("unique_term_sequence_per_session") {
                    return AppError::bad_request(anyhow::anyhow!(
                        "A term with this sequence already exists in this session"
                    ));
                }
            }
            AppError::from(e)
        })?;

        Ok(term)
    }

    /// Update a term with school filtering.
    #[instrument(skip(db))]
    pub async fn update_term_with_school_filter(
        db: &PgPool,
        term_id: TermId,
        school_id: SchoolId,
        dto: UpdateTermDto,
    ) -> Result<Term, AppError> {
        // Verify term belongs to school
        let term_exists = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(
                SELECT 1 FROM terms t
                JOIN academic_sessions s ON s.id = t.academic_session_id
                WHERE t.id = $1 AND s.school_id = $2
            )"#,
        )
        .bind(term_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        if !term_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Term not found")));
        }

        Self::update_term(db, term_id, dto).await
    }

    /// Delete a term.
    #[instrument(skip(db))]
    pub async fn delete_term(db: &PgPool, term_id: TermId) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM terms WHERE id = $1")
            .bind(term_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Term not found")));
        }

        Ok(())
    }

    /// Delete a term with school filtering.
    #[instrument(skip(db))]
    pub async fn delete_term_with_school_filter(
        db: &PgPool,
        term_id: TermId,
        school_id: SchoolId,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"DELETE FROM terms t
               USING academic_sessions s
               WHERE t.academic_session_id = s.id AND t.id = $1 AND s.school_id = $2"#,
        )
        .bind(term_id)
        .bind(school_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Term not found")));
        }

        Ok(())
    }

    /// Set a term as the current term.
    ///
    /// This will unset any other current term in the same session.
    /// The session must be active for a term to be marked as current.
    #[instrument(skip(db))]
    pub async fn set_current_term(db: &PgPool, term_id: TermId) -> Result<Term, AppError> {
        // Get the term with session info
        let term_info = sqlx::query_as::<_, TermWithSessionInfo>(
            r#"SELECT
                t.id,
                t.name,
                t.description,
                t.academic_session_id,
                t.start_date,
                t.end_date,
                t.sequence,
                t.is_current,
                t.created_at,
                t.updated_at,
                s.name as session_name,
                s.school_id,
                s.is_active as session_is_active
               FROM terms t
               JOIN academic_sessions s ON s.id = t.academic_session_id
               WHERE t.id = $1"#,
        )
        .bind(term_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Term not found")))?;

        // Verify the session is active
        if !term_info.session_is_active {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Cannot set current term: the academic session is not active"
            )));
        }

        // Unset current term for this session
        sqlx::query("UPDATE terms SET is_current = FALSE, updated_at = NOW() WHERE academic_session_id = $1")
            .bind(term_info.academic_session_id)
            .execute(db)
            .await?;

        // Set this term as current
        let term = sqlx::query_as::<_, Term>(
            r#"UPDATE terms
               SET is_current = TRUE, updated_at = NOW()
               WHERE id = $1
               RETURNING id, name, description, academic_session_id, start_date, end_date, sequence, is_current, created_at, updated_at"#,
        )
        .bind(term_id)
        .fetch_one(db)
        .await?;

        Ok(term)
    }

    /// Set a term as the current term with school filtering.
    #[instrument(skip(db))]
    pub async fn set_current_term_with_school_filter(
        db: &PgPool,
        term_id: TermId,
        school_id: SchoolId,
    ) -> Result<Term, AppError> {
        // Verify term belongs to school
        let term_exists = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(
                SELECT 1 FROM terms t
                JOIN academic_sessions s ON s.id = t.academic_session_id
                WHERE t.id = $1 AND s.school_id = $2
            )"#,
        )
        .bind(term_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        if !term_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Term not found")));
        }

        Self::set_current_term(db, term_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use chalkbyte_core::PaginationParams;
    use uuid::Uuid;

    use crate::modules::academic_sessions::model::CreateAcademicSessionDto;
    use crate::modules::academic_sessions::service::AcademicSessionService;

    async fn create_test_school(pool: &PgPool, name: &str) -> SchoolId {
        sqlx::query_scalar!(
            r#"INSERT INTO schools (name, address) VALUES ($1, $2) RETURNING id"#,
            name,
            Some("Test Address")
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .into()
    }

    async fn create_test_session(pool: &PgPool, school_id: SchoolId) -> AcademicSessionId {
        let session = AcademicSessionService::create_academic_session(
            pool,
            school_id,
            CreateAcademicSessionDto {
                name: format!("Session {}", Uuid::new_v4()),
                description: None,
                school_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            },
        )
        .await
        .unwrap();
        session.id
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_term_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        let dto = CreateTermDto {
            name: "Fall Semester".to_string(),
            description: Some("First semester".to_string()),
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            sequence: Some(1),
        };

        let result = TermService::create_term(&pool, session_id, dto).await;

        assert!(result.is_ok());
        let term = result.unwrap();
        assert_eq!(term.name, "Fall Semester");
        assert_eq!(term.sequence, 1);
        assert!(!term.is_current);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_term_invalid_dates(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        let dto = CreateTermDto {
            name: "Invalid Term".to_string(),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            sequence: None,
        };

        let result = TermService::create_term(&pool, session_id, dto).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_term_outside_session_range(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Try to create term that ends after session ends
        let dto = CreateTermDto {
            name: "Invalid Term".to_string(),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 8, 30).unwrap(), // After session end
            sequence: None,
        };

        let result = TermService::create_term(&pool, session_id, dto).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_term_overlapping_dates(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Create first term
        TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Fall Semester".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
                sequence: Some(1),
            },
        )
        .await
        .unwrap();

        // Try to create overlapping term
        let dto = CreateTermDto {
            name: "Overlapping Term".to_string(),
            description: None,
            academic_session_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(), // Overlaps with Fall
            end_date: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            sequence: Some(2),
        };

        let result = TermService::create_term(&pool, session_id, dto).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(err.error.to_string().contains("overlap"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_auto_sequence(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Create first term without sequence
        let term1 = TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Term 1".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 11, 30).unwrap(),
                sequence: None,
            },
        )
        .await
        .unwrap();

        // Create second term without sequence
        let term2 = TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Term 2".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
                sequence: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(term1.sequence, 1);
        assert_eq!(term2.sequence, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_set_current_term(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Activate session first
        AcademicSessionService::activate_academic_session(&pool, session_id, school_id)
            .await
            .unwrap();

        let term = TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Fall Semester".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
                sequence: Some(1),
            },
        )
        .await
        .unwrap();

        let current = TermService::set_current_term(&pool, term.id).await.unwrap();
        assert!(current.is_current);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_set_current_term_inactive_session(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Don't activate session
        let term = TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Fall Semester".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
                sequence: Some(1),
            },
        )
        .await
        .unwrap();

        let result = TermService::set_current_term(&pool, term.id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(err.error.to_string().contains("not active"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_current_term(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // No current term initially
        let current = TermService::get_current_term(&pool, school_id)
            .await
            .unwrap();
        assert!(current.is_none());

        // Activate session and set current term
        AcademicSessionService::activate_academic_session(&pool, session_id, school_id)
            .await
            .unwrap();

        let term = TermService::create_term(
            &pool,
            session_id,
            CreateTermDto {
                name: "Fall Semester".to_string(),
                description: None,
                academic_session_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 12, 20).unwrap(),
                sequence: Some(1),
            },
        )
        .await
        .unwrap();

        TermService::set_current_term(&pool, term.id).await.unwrap();

        let current = TermService::get_current_term(&pool, school_id)
            .await
            .unwrap();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, term.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_terms_pagination(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let session_id = create_test_session(&pool, school_id).await;

        // Create multiple terms with non-overlapping dates
        // Term 1: Sep 1-30, Term 2: Oct 1-31, Term 3: Nov 1-30, Term 4: Dec 1-31
        let term_dates = [
            (9, 1, 9, 30),   // Term 1: Sep
            (10, 1, 10, 31), // Term 2: Oct
            (11, 1, 11, 30), // Term 3: Nov
            (12, 1, 12, 31), // Term 4: Dec
        ];

        for (i, (start_month, start_day, end_month, end_day)) in term_dates.iter().enumerate() {
            TermService::create_term(
                &pool,
                session_id,
                CreateTermDto {
                    name: format!("Term {}", i + 1),
                    description: None,
                    academic_session_id: None,
                    start_date: NaiveDate::from_ymd_opt(2025, *start_month, *start_day).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2025, *end_month, *end_day).unwrap(),
                    sequence: Some((i + 1) as i32),
                },
            )
            .await
            .unwrap();
        }

        let filters = TermFilterParams {
            academic_session_id: None,
            is_current: None,
            school_id: None,
            pagination: PaginationParams {
                limit: Some(2),
                offset: Some(0),
                page: None,
            },
        };

        let result = TermService::get_terms_by_session(&pool, session_id, filters)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);
        assert_eq!(result.meta.total, 4);
        assert!(result.meta.has_more);
    }
}
