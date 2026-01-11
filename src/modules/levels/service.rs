use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::modules::levels::model::{
    AssignStudentsToLevelDto, BulkAssignResponse, CreateLevelDto, Level, LevelFilterParams,
    LevelWithStats, MoveStudentToLevelDto, PaginatedLevelsResponse, UpdateLevelDto,
};
use crate::modules::users::model::system_roles;
use crate::utils::errors::AppError;
use crate::utils::pagination::PaginationMeta;

pub struct LevelService;

impl LevelService {
    #[instrument]
    pub async fn create_level(
        db: &PgPool,
        school_id: Uuid,
        dto: CreateLevelDto,
    ) -> Result<Level, AppError> {
        let level = sqlx::query_as::<_, Level>(
            r#"INSERT INTO levels (name, description, school_id)
               VALUES ($1, $2, $3)
               RETURNING id, name, description, school_id, created_at, updated_at"#,
        )
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(school_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "A level with this name already exists in this school"
                ));
            }
            AppError::from(e)
        })?;

        Ok(level)
    }

    #[instrument]
    pub async fn get_levels_by_school(
        db: &PgPool,
        school_id: Uuid,
        filters: LevelFilterParams,
    ) -> Result<PaginatedLevelsResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let mut count_query = String::from("SELECT COUNT(*) FROM levels WHERE school_id = $1");
        let mut where_clause = String::new();
        let mut params = Vec::new();

        if let Some(name) = &filters.name {
            params.push(format!("%{}%", name));
            where_clause.push_str(&format!(" AND name ILIKE ${}", params.len() + 1));
        }

        count_query.push_str(&where_clause);

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query).bind(school_id);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await?;

        let student_role_id = system_roles::STUDENT;
        let mut data_query = String::from(
            r#"SELECT
                l.id,
                l.name,
                l.description,
                l.school_id,
                l.created_at,
                l.updated_at,
                COUNT(DISTINCT u.id) as student_count
               FROM levels l
               LEFT JOIN users u ON u.level_id = l.id
               LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = '"#,
        );
        data_query.push_str(&student_role_id.to_string());
        data_query.push_str("' WHERE l.school_id = $1");
        data_query.push_str(&where_clause);
        data_query.push_str(
            " GROUP BY l.id, l.name, l.description, l.school_id, l.created_at, l.updated_at",
        );
        data_query.push_str(" ORDER BY l.created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, LevelWithStats>(&data_query).bind(school_id);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let levels = data_sql.fetch_all(db).await?;

        let has_more = offset + limit < total;

        Ok(PaginatedLevelsResponse {
            data: levels,
            meta: PaginationMeta {
                total,
                limit,
                offset: Some(offset),
                page: None,
                has_more,
            },
        })
    }

    #[instrument]
    pub async fn get_level_by_id(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
    ) -> Result<LevelWithStats, AppError> {
        let student_role_id = system_roles::STUDENT;
        let level = sqlx::query_as::<_, LevelWithStats>(
            r#"SELECT
                l.id,
                l.name,
                l.description,
                l.school_id,
                l.created_at,
                l.updated_at,
                COUNT(DISTINCT u.id) as student_count
               FROM levels l
               LEFT JOIN users u ON u.level_id = l.id
               LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $3
               WHERE l.id = $1 AND l.school_id = $2
               GROUP BY l.id, l.name, l.description, l.school_id, l.created_at, l.updated_at"#,
        )
        .bind(level_id)
        .bind(school_id)
        .bind(student_role_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        Ok(level)
    }

    /// Get level by ID without school filtering (for system admins)
    #[instrument]
    pub async fn get_level_by_id_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
    ) -> Result<LevelWithStats, AppError> {
        let student_role_id = system_roles::STUDENT;
        let level = sqlx::query_as::<_, LevelWithStats>(
            r#"SELECT
                l.id,
                l.name,
                l.description,
                l.school_id,
                l.created_at,
                l.updated_at,
                COUNT(DISTINCT u.id) as student_count
               FROM levels l
               LEFT JOIN users u ON u.level_id = l.id
               LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $2
               WHERE l.id = $1
               GROUP BY l.id, l.name, l.description, l.school_id, l.created_at, l.updated_at"#,
        )
        .bind(level_id)
        .bind(student_role_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        Ok(level)
    }

    #[instrument]
    pub async fn update_level(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
        dto: UpdateLevelDto,
    ) -> Result<Level, AppError> {
        let existing_level = sqlx::query_as::<_, Level>(
            "SELECT id, name, description, school_id, created_at, updated_at FROM levels WHERE id = $1 AND school_id = $2",
        )
        .bind(level_id)
        .bind(school_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        let name = dto.name.unwrap_or(existing_level.name);
        let description = if dto.description.is_some() {
            dto.description
        } else {
            existing_level.description
        };

        let level = sqlx::query_as::<_, Level>(
            r#"UPDATE levels
               SET name = $1, description = $2, updated_at = NOW()
               WHERE id = $3 AND school_id = $4
               RETURNING id, name, description, school_id, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&description)
        .bind(level_id)
        .bind(school_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "A level with this name already exists in this school"
                ));
            }
            AppError::from(e)
        })?;

        Ok(level)
    }

    /// Update level without school filtering (for system admins)
    #[instrument]
    pub async fn update_level_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
        dto: UpdateLevelDto,
    ) -> Result<Level, AppError> {
        let existing_level = sqlx::query_as::<_, Level>(
            "SELECT id, name, description, school_id, created_at, updated_at FROM levels WHERE id = $1",
        )
        .bind(level_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        let name = dto.name.unwrap_or(existing_level.name);
        let description = if dto.description.is_some() {
            dto.description
        } else {
            existing_level.description
        };

        let level = sqlx::query_as::<_, Level>(
            r#"UPDATE levels
               SET name = $1, description = $2, updated_at = NOW()
               WHERE id = $3
               RETURNING id, name, description, school_id, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&description)
        .bind(level_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "A level with this name already exists in this school"
                ));
            }
            AppError::from(e)
        })?;

        Ok(level)
    }

    #[instrument]
    pub async fn delete_level(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM levels WHERE id = $1 AND school_id = $2")
            .bind(level_id)
            .bind(school_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        Ok(())
    }

    /// Delete level without school filtering (for system admins)
    #[instrument]
    pub async fn delete_level_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM levels WHERE id = $1")
            .bind(level_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        Ok(())
    }

    #[instrument]
    pub async fn assign_students_to_level(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
        dto: AssignStudentsToLevelDto,
    ) -> Result<BulkAssignResponse, AppError> {
        let level_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1 AND school_id = $2)",
        )
        .bind(level_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        if !level_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let mut assigned_count = 0;
        let mut failed_ids = Vec::new();

        let student_role_id = system_roles::STUDENT;
        for student_id in dto.student_ids {
            // First check if user is a student (has student role)
            let is_student = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
            )
            .bind(student_id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
            .unwrap_or(false);

            if !is_student {
                failed_ids.push(student_id);
                continue;
            }

            let result = sqlx::query(
                r#"UPDATE users
                   SET level_id = $1, updated_at = NOW()
                   WHERE id = $2 AND school_id = $3"#,
            )
            .bind(level_id)
            .bind(student_id)
            .bind(school_id)
            .execute(db)
            .await;

            match result {
                Ok(res) if res.rows_affected() > 0 => assigned_count += 1,
                _ => failed_ids.push(student_id),
            }
        }

        Ok(BulkAssignResponse {
            assigned_count,
            failed_ids,
        })
    }

    /// Assign students to level without school filtering (for system admins)
    #[instrument]
    pub async fn assign_students_to_level_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
        dto: AssignStudentsToLevelDto,
    ) -> Result<BulkAssignResponse, AppError> {
        let level_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1)")
                .bind(level_id)
                .fetch_one(db)
                .await?;

        if !level_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let mut assigned_count = 0;
        let mut failed_ids = Vec::new();

        let student_role_id = system_roles::STUDENT;
        for student_id in dto.student_ids {
            let is_student = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
            )
            .bind(student_id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
            .unwrap_or(false);

            if !is_student {
                failed_ids.push(student_id);
                continue;
            }

            let result =
                sqlx::query(r#"UPDATE users SET level_id = $1, updated_at = NOW() WHERE id = $2"#)
                    .bind(level_id)
                    .bind(student_id)
                    .execute(db)
                    .await;

            match result {
                Ok(res) if res.rows_affected() > 0 => assigned_count += 1,
                _ => failed_ids.push(student_id),
            }
        }

        Ok(BulkAssignResponse {
            assigned_count,
            failed_ids,
        })
    }

    #[instrument]
    pub async fn move_student_to_level(
        db: &PgPool,
        student_id: Uuid,
        school_id: Uuid,
        dto: MoveStudentToLevelDto,
    ) -> Result<(), AppError> {
        if let Some(new_level_id) = dto.level_id {
            let level_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1 AND school_id = $2)",
            )
            .bind(new_level_id)
            .bind(school_id)
            .fetch_one(db)
            .await?;

            if !level_exists {
                return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
            }
        }

        // Check if user is a student
        let student_role_id = system_roles::STUDENT;
        let is_student = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
        )
        .bind(student_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await?;

        if !is_student {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Student not found or not in this school"
            )));
        }

        let result = sqlx::query(
            r#"UPDATE users
               SET level_id = $1, updated_at = NOW()
               WHERE id = $2 AND school_id = $3"#,
        )
        .bind(dto.level_id)
        .bind(student_id)
        .bind(school_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Student not found or not in this school"
            )));
        }

        Ok(())
    }

    /// Move student to level without school filtering (for system admins)
    #[instrument]
    pub async fn move_student_to_level_no_school_filter(
        db: &PgPool,
        student_id: Uuid,
        dto: MoveStudentToLevelDto,
    ) -> Result<(), AppError> {
        if let Some(new_level_id) = dto.level_id {
            let level_exists =
                sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1)")
                    .bind(new_level_id)
                    .fetch_one(db)
                    .await?;

            if !level_exists {
                return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
            }
        }

        let student_role_id = system_roles::STUDENT;
        let is_student = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
        )
        .bind(student_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await?;

        if !is_student {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        let result =
            sqlx::query(r#"UPDATE users SET level_id = $1, updated_at = NOW() WHERE id = $2"#)
                .bind(dto.level_id)
                .bind(student_id)
                .execute(db)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }

    #[instrument]
    pub async fn get_students_in_level(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
    ) -> Result<Vec<crate::modules::users::model::User>, AppError> {
        let level_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1 AND school_id = $2)",
        )
        .bind(level_id)
        .bind(school_id)
        .fetch_one(db)
        .await?;

        if !level_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let student_role_id = system_roles::STUDENT;
        let students = sqlx::query_as::<_, crate::modules::users::model::User>(
            r#"SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.level_id, u.branch_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
               FROM users u
               INNER JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $3
               WHERE u.level_id = $1 AND u.school_id = $2
               ORDER BY u.last_name, u.first_name"#,
        )
        .bind(level_id)
        .bind(school_id)
        .bind(student_role_id)
        .fetch_all(db)
        .await?;

        Ok(students)
    }

    /// Get students in level without school filtering (for system admins)
    #[instrument]
    pub async fn get_students_in_level_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
    ) -> Result<Vec<crate::modules::users::model::User>, AppError> {
        let level_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM levels WHERE id = $1)")
                .bind(level_id)
                .fetch_one(db)
                .await?;

        if !level_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let student_role_id = system_roles::STUDENT;
        let students = sqlx::query_as::<_, crate::modules::users::model::User>(
            r#"SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.level_id, u.branch_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
               FROM users u
               INNER JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $2
               WHERE u.level_id = $1
               ORDER BY u.last_name, u.first_name"#,
        )
        .bind(level_id)
        .bind(student_role_id)
        .fetch_all(db)
        .await?;

        Ok(students)
    }

    #[instrument]
    pub async fn remove_student_from_level(
        db: &PgPool,
        student_id: Uuid,
        school_id: Uuid,
    ) -> Result<(), AppError> {
        // Check if user is a student
        let student_role_id = system_roles::STUDENT;
        let is_student = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
        )
        .bind(student_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await?;

        if !is_student {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Student not found or not in this school"
            )));
        }

        let result = sqlx::query(
            r#"UPDATE users
               SET level_id = NULL, updated_at = NOW()
               WHERE id = $1 AND school_id = $2"#,
        )
        .bind(student_id)
        .bind(school_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!(
                "Student not found or not in this school"
            )));
        }

        Ok(())
    }

    /// Remove student from level without school filtering (for system admins)
    #[instrument]
    pub async fn remove_student_from_level_no_school_filter(
        db: &PgPool,
        student_id: Uuid,
    ) -> Result<(), AppError> {
        let student_role_id = system_roles::STUDENT;
        let is_student = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
        )
        .bind(student_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await?;

        if !is_student {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        let result =
            sqlx::query(r#"UPDATE users SET level_id = NULL, updated_at = NOW() WHERE id = $1"#)
                .bind(student_id)
                .execute(db)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::pagination::PaginationParams;
    use axum::http::StatusCode;

    async fn create_test_school(pool: &PgPool, name: &str) -> Uuid {
        sqlx::query_scalar!(
            r#"INSERT INTO schools (name, address) VALUES ($1, $2) RETURNING id"#,
            name,
            Some("Test Address")
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn create_test_student(pool: &PgPool, school_id: Uuid, email: &str) -> Uuid {
        let user_id = sqlx::query_scalar!(
            r#"INSERT INTO users (first_name, last_name, email, password, school_id)
               VALUES ('Test', 'Student', $1, 'hashed', $2) RETURNING id"#,
            email,
            school_id
        )
        .fetch_one(pool)
        .await
        .unwrap();

        // Assign student role
        sqlx::query!(
            "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)",
            user_id,
            crate::modules::users::model::system_roles::STUDENT
        )
        .execute(pool)
        .await
        .unwrap();

        user_id
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_level_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: Some("Tenth grade level".to_string()),
            school_id: None,
        };

        let result = LevelService::create_level(&pool, school_id, dto).await;

        assert!(result.is_ok());
        let level = result.unwrap();
        assert_eq!(level.name, "Grade 10");
        assert_eq!(level.description, Some("Tenth grade level".to_string()));
        assert_eq!(level.school_id, school_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_level_duplicate_name_same_school(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let dto2 = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        let result = LevelService::create_level(&pool, school_id, dto2).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_level_same_name_different_schools(pool: PgPool) {
        let school1_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let school2_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        let dto2 = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        let result1 = LevelService::create_level(&pool, school1_id, dto).await;
        let result2 = LevelService::create_level(&pool, school2_id, dto2).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_levels_by_school(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto1 = CreateLevelDto {
            name: "Grade 9".to_string(),
            description: None,
            school_id: None,
        };
        let dto2 = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        LevelService::create_level(&pool, school_id, dto1)
            .await
            .unwrap();
        LevelService::create_level(&pool, school_id, dto2)
            .await
            .unwrap();

        let filters = LevelFilterParams {
            name: None,
            school_id: None,
            pagination: PaginationParams {
                limit: Some(10),
                offset: Some(0),
                page: None,
            },
        };

        let result = LevelService::get_levels_by_school(&pool, school_id, filters).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.meta.total, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_levels_filtered_by_name(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto1 = CreateLevelDto {
            name: "Primary Grade 1".to_string(),
            description: None,
            school_id: None,
        };
        let dto2 = CreateLevelDto {
            name: "Secondary Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        LevelService::create_level(&pool, school_id, dto1)
            .await
            .unwrap();
        LevelService::create_level(&pool, school_id, dto2)
            .await
            .unwrap();

        let filters = LevelFilterParams {
            name: Some("Primary".to_string()),
            school_id: None,
            pagination: PaginationParams {
                limit: Some(10),
                offset: Some(0),
                page: None,
            },
        };

        let result = LevelService::get_levels_by_school(&pool, school_id, filters).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].name, "Primary Grade 1");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_levels_pagination(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        for i in 1..=5 {
            let dto = CreateLevelDto {
                name: format!("Grade {}", i),
                description: None,
                school_id: None,
            };
            LevelService::create_level(&pool, school_id, dto)
                .await
                .unwrap();
        }

        let filters = LevelFilterParams {
            name: None,
            school_id: None,
            pagination: PaginationParams {
                limit: Some(2),
                offset: Some(0),
                page: None,
            },
        };

        let result = LevelService::get_levels_by_school(&pool, school_id, filters).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.meta.total, 5);
        assert!(response.meta.has_more);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_level_by_id_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: Some("Test description".to_string()),
            school_id: None,
        };

        let created = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let result = LevelService::get_level_by_id(&pool, created.id, school_id).await;

        assert!(result.is_ok());
        let level = result.unwrap();
        assert_eq!(level.id, created.id);
        assert_eq!(level.name, "Grade 10");
        assert_eq!(level.student_count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_level_by_id_not_found(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_id = Uuid::new_v4();

        let result = LevelService::get_level_by_id(&pool, random_id, school_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_level_by_id_different_school(pool: PgPool) {
        let school1_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let school2_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        let created = LevelService::create_level(&pool, school1_id, dto)
            .await
            .unwrap();

        let result = LevelService::get_level_by_id(&pool, created.id, school2_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_level_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: Some("Original description".to_string()),
            school_id: None,
        };

        let created = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let update_dto = UpdateLevelDto {
            name: Some("Grade 11".to_string()),
            description: Some("Updated description".to_string()),
        };

        let result = LevelService::update_level(&pool, created.id, school_id, update_dto).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.name, "Grade 11");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_level_partial(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: Some("Original description".to_string()),
            school_id: None,
        };

        let created = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let update_dto = UpdateLevelDto {
            name: Some("Grade 11".to_string()),
            description: None,
        };

        let result = LevelService::update_level(&pool, created.id, school_id, update_dto).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.name, "Grade 11");
        assert_eq!(
            updated.description,
            Some("Original description".to_string())
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_level_not_found(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_id = Uuid::new_v4();

        let update_dto = UpdateLevelDto {
            name: Some("Grade 11".to_string()),
            description: None,
        };

        let result = LevelService::update_level(&pool, random_id, school_id, update_dto).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_level_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };

        let created = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let result = LevelService::delete_level(&pool, created.id, school_id).await;

        assert!(result.is_ok());

        let get_result = LevelService::get_level_by_id(&pool, created.id, school_id).await;
        assert!(get_result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_level_not_found(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_id = Uuid::new_v4();

        let result = LevelService::delete_level(&pool, random_id, school_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_assign_students_to_level_success(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };
        let level = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let student1_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;
        let student2_id =
            create_test_student(&pool, school_id, &format!("s2-{}@test.com", Uuid::new_v4())).await;

        let assign_dto = AssignStudentsToLevelDto {
            student_ids: vec![student1_id, student2_id],
        };

        let result =
            LevelService::assign_students_to_level(&pool, level.id, school_id, assign_dto).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.assigned_count, 2);
        assert_eq!(response.failed_ids.len(), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_assign_students_with_invalid_ids(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let dto = CreateLevelDto {
            name: "Grade 10".to_string(),
            description: None,
            school_id: None,
        };
        let level = LevelService::create_level(&pool, school_id, dto)
            .await
            .unwrap();

        let student_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;
        let invalid_id = Uuid::new_v4();

        let assign_dto = AssignStudentsToLevelDto {
            student_ids: vec![student_id, invalid_id],
        };

        let result =
            LevelService::assign_students_to_level(&pool, level.id, school_id, assign_dto).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.assigned_count, 1);
        assert_eq!(response.failed_ids.len(), 1);
        assert_eq!(response.failed_ids[0], invalid_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_assign_students_to_nonexistent_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_level_id = Uuid::new_v4();

        let student_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;

        let assign_dto = AssignStudentsToLevelDto {
            student_ids: vec![student_id],
        };

        let result =
            LevelService::assign_students_to_level(&pool, random_level_id, school_id, assign_dto)
                .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_move_student_to_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let level1 = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 9".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let level2 = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 10".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let student_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;

        LevelService::assign_students_to_level(
            &pool,
            level1.id,
            school_id,
            AssignStudentsToLevelDto {
                student_ids: vec![student_id],
            },
        )
        .await
        .unwrap();

        let move_dto = MoveStudentToLevelDto {
            level_id: Some(level2.id),
        };

        let result =
            LevelService::move_student_to_level(&pool, student_id, school_id, move_dto).await;

        assert!(result.is_ok());

        let students = LevelService::get_students_in_level(&pool, level2.id, school_id)
            .await
            .unwrap();
        assert_eq!(students.len(), 1);
        assert_eq!(students[0].id, student_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_move_student_to_null_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let level = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 10".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let student_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;

        LevelService::assign_students_to_level(
            &pool,
            level.id,
            school_id,
            AssignStudentsToLevelDto {
                student_ids: vec![student_id],
            },
        )
        .await
        .unwrap();

        let move_dto = MoveStudentToLevelDto { level_id: None };

        let result =
            LevelService::move_student_to_level(&pool, student_id, school_id, move_dto).await;

        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_students_in_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let level = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 10".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let student1_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;
        let student2_id =
            create_test_student(&pool, school_id, &format!("s2-{}@test.com", Uuid::new_v4())).await;

        LevelService::assign_students_to_level(
            &pool,
            level.id,
            school_id,
            AssignStudentsToLevelDto {
                student_ids: vec![student1_id, student2_id],
            },
        )
        .await
        .unwrap();

        let result = LevelService::get_students_in_level(&pool, level.id, school_id).await;

        assert!(result.is_ok());
        let students = result.unwrap();
        assert_eq!(students.len(), 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_students_in_nonexistent_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_level_id = Uuid::new_v4();

        let result = LevelService::get_students_in_level(&pool, random_level_id, school_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_remove_student_from_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let level = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 10".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let student_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;

        LevelService::assign_students_to_level(
            &pool,
            level.id,
            school_id,
            AssignStudentsToLevelDto {
                student_ids: vec![student_id],
            },
        )
        .await
        .unwrap();

        let result = LevelService::remove_student_from_level(&pool, student_id, school_id).await;

        assert!(result.is_ok());

        let students = LevelService::get_students_in_level(&pool, level.id, school_id)
            .await
            .unwrap();
        assert_eq!(students.len(), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_remove_nonexistent_student_from_level(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;
        let random_student_id = Uuid::new_v4();

        let result =
            LevelService::remove_student_from_level(&pool, random_student_id, school_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_level_student_count_updates(pool: PgPool) {
        let school_id = create_test_school(&pool, &format!("School {}", Uuid::new_v4())).await;

        let level = LevelService::create_level(
            &pool,
            school_id,
            CreateLevelDto {
                name: "Grade 10".to_string(),
                description: None,
                school_id: None,
            },
        )
        .await
        .unwrap();

        let student1_id =
            create_test_student(&pool, school_id, &format!("s1-{}@test.com", Uuid::new_v4())).await;
        let student2_id =
            create_test_student(&pool, school_id, &format!("s2-{}@test.com", Uuid::new_v4())).await;

        let level_with_stats = LevelService::get_level_by_id(&pool, level.id, school_id)
            .await
            .unwrap();
        assert_eq!(level_with_stats.student_count, 0);

        LevelService::assign_students_to_level(
            &pool,
            level.id,
            school_id,
            AssignStudentsToLevelDto {
                student_ids: vec![student1_id, student2_id],
            },
        )
        .await
        .unwrap();

        let level_with_stats = LevelService::get_level_by_id(&pool, level.id, school_id)
            .await
            .unwrap();
        assert_eq!(level_with_stats.student_count, 2);

        LevelService::remove_student_from_level(&pool, student1_id, school_id)
            .await
            .unwrap();

        let level_with_stats = LevelService::get_level_by_id(&pool, level.id, school_id)
            .await
            .unwrap();
        assert_eq!(level_with_stats.student_count, 1);
    }
}
