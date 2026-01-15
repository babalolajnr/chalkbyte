use sqlx::PgPool;
use tracing::instrument;

use chalkbyte_core::{AppError, PaginationMeta};
use chalkbyte_models::ids::{AcademicSessionId, SchoolId};

use crate::modules::academic_sessions::model::{
    AcademicSession, AcademicSessionFilterParams, AcademicSessionWithStats,
    CreateAcademicSessionDto, PaginatedAcademicSessionsResponse, UpdateAcademicSessionDto,
};

pub struct AcademicSessionService;

impl AcademicSessionService {
    /// Create a new academic session.
    ///
    /// Validates that:
    /// - start_date < end_date
    /// - Session name is unique within the school
    #[instrument(skip(db))]
    pub async fn create_academic_session(
        db: &PgPool,
        school_id: SchoolId,
        dto: CreateAcademicSessionDto,
    ) -> Result<AcademicSession, AppError> {
        // Validate dates
        if dto.start_date >= dto.end_date {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Start date must be before end date"
            )));
        }

        let session = sqlx::query_as::<_, AcademicSession>(
            r#"INSERT INTO academic_sessions (name, description, school_id, start_date, end_date)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(school_id)
        .bind(dto.start_date)
        .bind(dto.end_date)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::bad_request(anyhow::anyhow!(
                        "An academic session with this name already exists in this school"
                    ));
                }
            }
            AppError::from(e)
        })?;

        Ok(session)
    }

    /// Get paginated list of academic sessions for a school.
    #[instrument(skip(db))]
    pub async fn get_academic_sessions_by_school(
        db: &PgPool,
        school_id: SchoolId,
        filters: AcademicSessionFilterParams,
    ) -> Result<PaginatedAcademicSessionsResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let mut count_query =
            String::from("SELECT COUNT(*) FROM academic_sessions WHERE school_id = $1");
        let mut where_clause = String::new();

        if let Some(is_active) = filters.is_active {
            where_clause.push_str(&format!(" AND is_active = {}", is_active));
        }

        count_query.push_str(&where_clause);

        let total = sqlx::query_scalar::<_, i64>(&count_query)
            .bind(school_id)
            .fetch_one(db)
            .await?;

        let mut data_query = String::from(
            r#"SELECT
                s.id,
                s.name,
                s.description,
                s.school_id,
                s.start_date,
                s.end_date,
                s.is_active,
                s.created_at,
                s.updated_at,
                COUNT(t.id) as term_count
               FROM academic_sessions s
               LEFT JOIN terms t ON t.academic_session_id = s.id
               WHERE s.school_id = $1"#,
        );
        data_query.push_str(&where_clause);
        data_query.push_str(
            " GROUP BY s.id, s.name, s.description, s.school_id, s.start_date, s.end_date, s.is_active, s.created_at, s.updated_at",
        );
        data_query.push_str(" ORDER BY s.start_date DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let sessions = sqlx::query_as::<_, AcademicSessionWithStats>(&data_query)
            .bind(school_id)
            .fetch_all(db)
            .await?;

        let has_more = offset + limit < total;

        Ok(PaginatedAcademicSessionsResponse {
            data: sessions,
            meta: PaginationMeta {
                total,
                limit,
                offset: Some(offset),
                page: None,
                has_more,
            },
        })
    }

    /// Get an academic session by ID with school filtering.
    #[instrument(skip(db))]
    pub async fn get_academic_session_by_id(
        db: &PgPool,
        session_id: AcademicSessionId,
        school_id: SchoolId,
    ) -> Result<AcademicSessionWithStats, AppError> {
        let session = sqlx::query_as::<_, AcademicSessionWithStats>(
            r#"SELECT
                s.id,
                s.name,
                s.description,
                s.school_id,
                s.start_date,
                s.end_date,
                s.is_active,
                s.created_at,
                s.updated_at,
                COUNT(t.id) as term_count
               FROM academic_sessions s
               LEFT JOIN terms t ON t.academic_session_id = s.id
               WHERE s.id = $1 AND s.school_id = $2
               GROUP BY s.id, s.name, s.description, s.school_id, s.start_date, s.end_date, s.is_active, s.created_at, s.updated_at"#,
        )
        .bind(session_id)
        .bind(school_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        Ok(session)
    }

    /// Get an academic session by ID without school filtering (for system admins).
    #[instrument(skip(db))]
    pub async fn get_academic_session_by_id_no_school_filter(
        db: &PgPool,
        session_id: AcademicSessionId,
    ) -> Result<AcademicSessionWithStats, AppError> {
        let session = sqlx::query_as::<_, AcademicSessionWithStats>(
            r#"SELECT
                s.id,
                s.name,
                s.description,
                s.school_id,
                s.start_date,
                s.end_date,
                s.is_active,
                s.created_at,
                s.updated_at,
                COUNT(t.id) as term_count
               FROM academic_sessions s
               LEFT JOIN terms t ON t.academic_session_id = s.id
               WHERE s.id = $1
               GROUP BY s.id, s.name, s.description, s.school_id, s.start_date, s.end_date, s.is_active, s.created_at, s.updated_at"#,
        )
        .bind(session_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        Ok(session)
    }

    /// Get the active academic session for a school.
    #[instrument(skip(db))]
    pub async fn get_active_academic_session(
        db: &PgPool,
        school_id: SchoolId,
    ) -> Result<Option<AcademicSessionWithStats>, AppError> {
        let session = sqlx::query_as::<_, AcademicSessionWithStats>(
            r#"SELECT
                s.id,
                s.name,
                s.description,
                s.school_id,
                s.start_date,
                s.end_date,
                s.is_active,
                s.created_at,
                s.updated_at,
                COUNT(t.id) as term_count
               FROM academic_sessions s
               LEFT JOIN terms t ON t.academic_session_id = s.id
               WHERE s.school_id = $1 AND s.is_active = TRUE
               GROUP BY s.id, s.name, s.description, s.school_id, s.start_date, s.end_date, s.is_active, s.created_at, s.updated_at"#,
        )
        .bind(school_id)
        .fetch_optional(db)
        .await?;

        Ok(session)
    }

    /// Update an academic session.
    ///
    /// Validates that:
    /// - start_date < end_date (if dates are being updated)
    /// - If session has terms, date changes don't invalidate term dates
    #[instrument(skip(db))]
    pub async fn update_academic_session(
        db: &PgPool,
        session_id: AcademicSessionId,
        school_id: SchoolId,
        dto: UpdateAcademicSessionDto,
    ) -> Result<AcademicSession, AppError> {
        let existing = sqlx::query_as::<_, AcademicSession>(
            r#"SELECT id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at
               FROM academic_sessions WHERE id = $1 AND school_id = $2"#,
        )
        .bind(session_id)
        .bind(school_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        let name = dto.name.unwrap_or(existing.name);
        let description = if dto.description.is_some() {
            dto.description
        } else {
            existing.description
        };
        let start_date = dto.start_date.unwrap_or(existing.start_date);
        let end_date = dto.end_date.unwrap_or(existing.end_date);

        // Validate dates
        if start_date >= end_date {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Start date must be before end date"
            )));
        }

        // Check if any terms would fall outside the new date range
        let invalid_terms = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM terms
               WHERE academic_session_id = $1
               AND (start_date < $2 OR end_date > $3)"#,
        )
        .bind(session_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db)
        .await?;

        if invalid_terms > 0 {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Cannot update dates: {} term(s) would fall outside the new date range",
                invalid_terms
            )));
        }

        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET name = $1, description = $2, start_date = $3, end_date = $4, updated_at = NOW()
               WHERE id = $5 AND school_id = $6
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&description)
        .bind(start_date)
        .bind(end_date)
        .bind(session_id)
        .bind(school_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::bad_request(anyhow::anyhow!(
                        "An academic session with this name already exists in this school"
                    ));
                }
            }
            AppError::from(e)
        })?;

        Ok(session)
    }

    /// Update an academic session without school filtering (for system admins).
    #[instrument(skip(db))]
    pub async fn update_academic_session_no_school_filter(
        db: &PgPool,
        session_id: AcademicSessionId,
        dto: UpdateAcademicSessionDto,
    ) -> Result<AcademicSession, AppError> {
        let existing = sqlx::query_as::<_, AcademicSession>(
            r#"SELECT id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at
               FROM academic_sessions WHERE id = $1"#,
        )
        .bind(session_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        let name = dto.name.unwrap_or(existing.name);
        let description = if dto.description.is_some() {
            dto.description
        } else {
            existing.description
        };
        let start_date = dto.start_date.unwrap_or(existing.start_date);
        let end_date = dto.end_date.unwrap_or(existing.end_date);

        // Validate dates
        if start_date >= end_date {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Start date must be before end date"
            )));
        }

        // Check if any terms would fall outside the new date range
        let invalid_terms = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM terms
               WHERE academic_session_id = $1
               AND (start_date < $2 OR end_date > $3)"#,
        )
        .bind(session_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db)
        .await?;

        if invalid_terms > 0 {
            return Err(AppError::bad_request(anyhow::anyhow!(
                "Cannot update dates: {} term(s) would fall outside the new date range",
                invalid_terms
            )));
        }

        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET name = $1, description = $2, start_date = $3, end_date = $4, updated_at = NOW()
               WHERE id = $5
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&description)
        .bind(start_date)
        .bind(end_date)
        .bind(session_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::bad_request(anyhow::anyhow!(
                        "An academic session with this name already exists in this school"
                    ));
                }
            }
            AppError::from(e)
        })?;

        Ok(session)
    }

    /// Delete an academic session.
    #[instrument(skip(db))]
    pub async fn delete_academic_session(
        db: &PgPool,
        session_id: AcademicSessionId,
        school_id: SchoolId,
    ) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM academic_sessions WHERE id = $1 AND school_id = $2")
            .bind(session_id)
            .bind(school_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Academic session not found"
            )));
        }

        Ok(())
    }

    /// Delete an academic session without school filtering (for system admins).
    #[instrument(skip(db))]
    pub async fn delete_academic_session_no_school_filter(
        db: &PgPool,
        session_id: AcademicSessionId,
    ) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM academic_sessions WHERE id = $1")
            .bind(session_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Academic session not found"
            )));
        }

        Ok(())
    }

    /// Activate an academic session.
    ///
    /// This will deactivate any currently active session for the school
    /// and activate the specified session.
    #[instrument(skip(db))]
    pub async fn activate_academic_session(
        db: &PgPool,
        session_id: AcademicSessionId,
        school_id: SchoolId,
    ) -> Result<AcademicSession, AppError> {
        // Verify the session exists and belongs to the school
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM academic_sessions WHERE id = $1 AND school_id = $2)",
        )
        .bind(session_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        if !exists {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Academic session not found"
            )));
        }

        // Deactivate all sessions for this school first
        sqlx::query("UPDATE academic_sessions SET is_active = FALSE, updated_at = NOW() WHERE school_id = $1")
            .bind(school_id)
            .execute(db)
            .await?;

        // Activate the specified session
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET is_active = TRUE, updated_at = NOW()
               WHERE id = $1 AND school_id = $2
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(session_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        Ok(session)
    }

    /// Activate an academic session without school filtering (for system admins).
    #[instrument(skip(db))]
    pub async fn activate_academic_session_no_school_filter(
        db: &PgPool,
        session_id: AcademicSessionId,
    ) -> Result<AcademicSession, AppError> {
        // Get the session to find its school_id
        let existing = sqlx::query_as::<_, AcademicSession>(
            r#"SELECT id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at
               FROM academic_sessions WHERE id = $1"#,
        )
        .bind(session_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        // Deactivate all sessions for this school first
        sqlx::query("UPDATE academic_sessions SET is_active = FALSE, updated_at = NOW() WHERE school_id = $1")
            .bind(existing.school_id)
            .execute(db)
            .await?;

        // Activate the specified session
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET is_active = TRUE, updated_at = NOW()
               WHERE id = $1
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(session_id)
        .fetch_one(db)
        .await?;

        Ok(session)
    }

    /// Deactivate an academic session.
    #[instrument(skip(db))]
    pub async fn deactivate_academic_session(
        db: &PgPool,
        session_id: AcademicSessionId,
        school_id: SchoolId,
    ) -> Result<AcademicSession, AppError> {
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET is_active = FALSE, updated_at = NOW()
               WHERE id = $1 AND school_id = $2
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(session_id)
        .bind(school_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        // Also unset any current term in this session
        sqlx::query("UPDATE terms SET is_current = FALSE, updated_at = NOW() WHERE academic_session_id = $1")
            .bind(session_id)
            .execute(db)
            .await?;

        Ok(session)
    }

    /// Deactivate an academic session without school filtering (for system admins).
    #[instrument(skip(db))]
    pub async fn deactivate_academic_session_no_school_filter(
        db: &PgPool,
        session_id: AcademicSessionId,
    ) -> Result<AcademicSession, AppError> {
        let session = sqlx::query_as::<_, AcademicSession>(
            r#"UPDATE academic_sessions
               SET is_active = FALSE, updated_at = NOW()
               WHERE id = $1
               RETURNING id, name, description, school_id, start_date, end_date, is_active, created_at, updated_at"#,
        )
        .bind(session_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Academic session not found")))?;

        // Also unset any current term in this session
        sqlx::query("UPDATE terms SET is_current = FALSE, updated_at = NOW() WHERE academic_session_id = $1")
            .bind(session_id)
            .execute(db)
            .await?;

        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use chalkbyte_core::PaginationParams;
    use chrono::NaiveDate;
    use uuid::Uuid;

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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_academic_session_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateAcademicSessionDto {
            name: "2025-2026 Academic Year".to_string(),
            description: Some("Main academic year".to_string()),
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        };

        let result = AcademicSessionService::create_academic_session(&pool, school_id, dto).await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.name, "2025-2026 Academic Year");
        assert!(!session.is_active);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_academic_session_invalid_dates(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateAcademicSessionDto {
            name: "Invalid Session".to_string(),
            description: None,
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
        };

        let result = AcademicSessionService::create_academic_session(&pool, school_id, dto).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_activate_academic_session(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateAcademicSessionDto {
            name: "2025-2026".to_string(),
            description: None,
            school_id: None,
            start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        };

        let session = AcademicSessionService::create_academic_session(&pool, school_id, dto)
            .await
            .unwrap();
        assert!(!session.is_active);

        let activated =
            AcademicSessionService::activate_academic_session(&pool, session.id, school_id)
                .await
                .unwrap();
        assert!(activated.is_active);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_only_one_active_session_per_school(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let session1 = AcademicSessionService::create_academic_session(
            &pool,
            school_id,
            CreateAcademicSessionDto {
                name: "2024-2025".to_string(),
                description: None,
                school_id: None,
                start_date: NaiveDate::from_ymd_opt(2024, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
            },
        )
        .await
        .unwrap();

        let session2 = AcademicSessionService::create_academic_session(
            &pool,
            school_id,
            CreateAcademicSessionDto {
                name: "2025-2026".to_string(),
                description: None,
                school_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            },
        )
        .await
        .unwrap();

        // Activate first session
        AcademicSessionService::activate_academic_session(&pool, session1.id, school_id)
            .await
            .unwrap();

        // Activate second session - should deactivate first
        AcademicSessionService::activate_academic_session(&pool, session2.id, school_id)
            .await
            .unwrap();

        // Check that only second session is active
        let s1 = AcademicSessionService::get_academic_session_by_id(&pool, session1.id, school_id)
            .await
            .unwrap();
        let s2 = AcademicSessionService::get_academic_session_by_id(&pool, session2.id, school_id)
            .await
            .unwrap();

        assert!(!s1.is_active);
        assert!(s2.is_active);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_active_academic_session(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        // No active session initially
        let active = AcademicSessionService::get_active_academic_session(&pool, school_id)
            .await
            .unwrap();
        assert!(active.is_none());

        // Create and activate a session
        let session = AcademicSessionService::create_academic_session(
            &pool,
            school_id,
            CreateAcademicSessionDto {
                name: "2025-2026".to_string(),
                description: None,
                school_id: None,
                start_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            },
        )
        .await
        .unwrap();

        AcademicSessionService::activate_academic_session(&pool, session.id, school_id)
            .await
            .unwrap();

        let active = AcademicSessionService::get_active_academic_session(&pool, school_id)
            .await
            .unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, session.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_academic_sessions_pagination(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        // Create multiple sessions
        for i in 2020..2025 {
            AcademicSessionService::create_academic_session(
                &pool,
                school_id,
                CreateAcademicSessionDto {
                    name: format!("{}-{}", i, i + 1),
                    description: None,
                    school_id: None,
                    start_date: NaiveDate::from_ymd_opt(i, 9, 1).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(i + 1, 6, 30).unwrap(),
                },
            )
            .await
            .unwrap();
        }

        let filters = AcademicSessionFilterParams {
            school_id: None,
            is_active: None,
            pagination: PaginationParams {
                limit: Some(2),
                offset: Some(0),
                page: None,
            },
        };

        let result =
            AcademicSessionService::get_academic_sessions_by_school(&pool, school_id, filters)
                .await
                .unwrap();

        assert_eq!(result.data.len(), 2);
        assert_eq!(result.meta.total, 5);
        assert!(result.meta.has_more);
    }
}
