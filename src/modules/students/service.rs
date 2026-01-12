use crate::{
    modules::students::model::{CreateStudentDto, Student, UpdateStudentDto},
    modules::users::model::system_roles,
    utils::{errors::AppError, password::hash_password},
};
use anyhow::Context;
use chalkbyte_models::Email;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

pub struct StudentService;

impl StudentService {
    #[instrument(skip(db, dto))]
    pub async fn create_student(
        db: &PgPool,
        dto: CreateStudentDto,
        school_id: Uuid,
    ) -> Result<Student, AppError> {
        let hashed_password = hash_password(&dto.password)?;

        // Insert user without role column
        let student = sqlx::query_as::<_, Student>(
            r#"
            INSERT INTO users (first_name, last_name, email, password, school_id, date_of_birth, grade_level)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, first_name, last_name, email, school_id, date_of_birth, grade_level, created_at, updated_at
            "#,
        )
        .bind(&dto.first_name)
        .bind(&dto.last_name)
        .bind(dto.email.as_str())
        .bind(&hashed_password)
        .bind(school_id)
        .bind(dto.date_of_birth)
        .bind(&dto.grade_level)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Student with email {} already exists",
                    dto.email
                ));
            }
            AppError::database(anyhow::Error::from(e))
        })?;

        // Assign student role via user_roles
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(student.id)
        .bind(system_roles::STUDENT)
        .execute(db)
        .await
        .map_err(|e| AppError::database(anyhow::Error::from(e)))?;

        Ok(student)
    }

    #[instrument(skip(db))]
    pub async fn get_students_by_school(
        db: &PgPool,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Student>, i64), AppError> {
        let student_role_id = system_roles::STUDENT;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.school_id = $1 AND ur.role_id = $2
            "#,
        )
        .bind(school_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await
        .context("Failed to count students by school")
        .map_err(AppError::database)?;

        let students = sqlx::query_as::<_, Student>(
            r#"
            SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.school_id = $1 AND ur.role_id = $2
            ORDER BY u.last_name, u.first_name
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(school_id)
        .bind(student_role_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .context("Failed to fetch students by school")
        .map_err(AppError::database)?;

        Ok((students, total))
    }

    #[instrument(skip(db))]
    pub async fn get_student_by_id(
        db: &PgPool,
        id: Uuid,
        school_id: Uuid,
    ) -> Result<Student, AppError> {
        let student_role_id = system_roles::STUDENT;

        // Check if user exists and is a student
        let student_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users u
                INNER JOIN user_roles ur ON ur.user_id = u.id
                WHERE u.id = $1 AND ur.role_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(student_role_id)
        .fetch_one(db)
        .await
        .context("Failed to check student existence")
        .map_err(AppError::database)?;

        if !student_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        // Check school ownership
        let student_school_id =
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
                .bind(id)
                .fetch_one(db)
                .await
                .context("Failed to fetch student school")
                .map_err(AppError::database)?;

        if student_school_id != Some(school_id) {
            return Err(AppError::forbidden(
                "Cannot access student from different school".to_string(),
            ));
        }

        let student = sqlx::query_as::<_, Student>(
            r#"
            SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.id = $1 AND u.school_id = $2 AND ur.role_id = $3
            "#,
        )
        .bind(id)
        .bind(school_id)
        .bind(student_role_id)
        .fetch_one(db)
        .await
        .context("Failed to fetch student by ID")
        .map_err(AppError::database)?;

        Ok(student)
    }

    #[instrument(skip(db, dto))]
    pub async fn update_student(
        db: &PgPool,
        id: Uuid,
        school_id: Uuid,
        dto: UpdateStudentDto,
    ) -> Result<Student, AppError> {
        let existing = Self::get_student_by_id(db, id, school_id).await?;

        let first_name = dto.first_name.unwrap_or(existing.first_name);
        let last_name = dto.last_name.unwrap_or(existing.last_name);
        let email: Email = dto.email.unwrap_or(existing.email);
        let date_of_birth = dto.date_of_birth.or(existing.date_of_birth);
        let grade_level = dto.grade_level.or(existing.grade_level);

        let student_role_id = system_roles::STUDENT;

        let updated_student = if let Some(password) = dto.password {
            let hashed_password = hash_password(&password)?;
            sqlx::query_as::<_, Student>(
                r#"
                UPDATE users u
                SET first_name = $1, last_name = $2, email = $3, password = $4, date_of_birth = $5, grade_level = $6, updated_at = NOW()
                FROM user_roles ur
                WHERE u.id = ur.user_id AND u.id = $7 AND u.school_id = $8 AND ur.role_id = $9
                RETURNING u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
                "#,
            )
            .bind(&first_name)
            .bind(&last_name)
            .bind(email.as_str())
            .bind(&hashed_password)
            .bind(date_of_birth)
            .bind(&grade_level)
            .bind(id)
            .bind(school_id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
        } else {
            sqlx::query_as::<_, Student>(
                r#"
                UPDATE users u
                SET first_name = $1, last_name = $2, email = $3, date_of_birth = $4, grade_level = $5, updated_at = NOW()
                FROM user_roles ur
                WHERE u.id = ur.user_id AND u.id = $6 AND u.school_id = $7 AND ur.role_id = $8
                RETURNING u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
                "#,
            )
            .bind(&first_name)
            .bind(&last_name)
            .bind(email.as_str())
            .bind(date_of_birth)
            .bind(&grade_level)
            .bind(id)
            .bind(school_id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
        }
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Student with email {} already exists",
                    email
                ));
            }
            AppError::database(anyhow::Error::from(e))
        })?;

        Ok(updated_student)
    }

    #[instrument(skip(db))]
    pub async fn delete_student(db: &PgPool, id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let student_role_id = system_roles::STUDENT;

        // Check if user exists and is a student
        let student_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users u
                INNER JOIN user_roles ur ON ur.user_id = u.id
                WHERE u.id = $1 AND ur.role_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(student_role_id)
        .fetch_one(db)
        .await
        .context("Failed to check student existence")
        .map_err(AppError::database)?;

        if !student_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        // Check school ownership
        let student_school_id =
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
                .bind(id)
                .fetch_one(db)
                .await
                .context("Failed to fetch student school")
                .map_err(AppError::database)?;

        if student_school_id != Some(school_id) {
            return Err(AppError::forbidden(
                "Cannot delete student from different school".to_string(),
            ));
        }

        // Delete role assignment first
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(id)
            .execute(db)
            .await
            .context("Failed to delete student role assignment")
            .map_err(AppError::database)?;

        // Delete the user
        sqlx::query("DELETE FROM users WHERE id = $1 AND school_id = $2")
            .bind(id)
            .bind(school_id)
            .execute(db)
            .await
            .context("Failed to delete student")
            .map_err(AppError::database)?;

        Ok(())
    }

    // ============ No School Filter Methods (for System Admins) ============

    #[instrument(skip(db))]
    pub async fn get_student_by_id_no_school_filter(
        db: &PgPool,
        id: Uuid,
    ) -> Result<Student, AppError> {
        let student_role_id = system_roles::STUDENT;

        let student = sqlx::query_as::<_, Student>(
            r#"
            SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.id = $1 AND ur.role_id = $2
            "#,
        )
        .bind(id)
        .bind(student_role_id)
        .fetch_optional(db)
        .await
        .context("Failed to fetch student by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Student not found")))?;

        Ok(student)
    }

    #[instrument(skip(db, dto))]
    pub async fn update_student_no_school_filter(
        db: &PgPool,
        id: Uuid,
        dto: UpdateStudentDto,
    ) -> Result<Student, AppError> {
        let existing = Self::get_student_by_id_no_school_filter(db, id).await?;

        let first_name = dto.first_name.unwrap_or(existing.first_name);
        let last_name = dto.last_name.unwrap_or(existing.last_name);
        let email: Email = dto.email.unwrap_or(existing.email);
        let date_of_birth = dto.date_of_birth.or(existing.date_of_birth);
        let grade_level = dto.grade_level.or(existing.grade_level);

        let student_role_id = system_roles::STUDENT;

        let updated_student = if let Some(password) = dto.password {
            let hashed_password = hash_password(&password)?;
            sqlx::query_as::<_, Student>(
                r#"
                UPDATE users u
                SET first_name = $1, last_name = $2, email = $3, password = $4, date_of_birth = $5, grade_level = $6, updated_at = NOW()
                FROM user_roles ur
                WHERE u.id = ur.user_id AND u.id = $7 AND ur.role_id = $8
                RETURNING u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
                "#,
            )
            .bind(&first_name)
            .bind(&last_name)
            .bind(email.as_str())
            .bind(&hashed_password)
            .bind(date_of_birth)
            .bind(&grade_level)
            .bind(id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
        } else {
            sqlx::query_as::<_, Student>(
                r#"
                UPDATE users u
                SET first_name = $1, last_name = $2, email = $3, date_of_birth = $4, grade_level = $5, updated_at = NOW()
                FROM user_roles ur
                WHERE u.id = ur.user_id AND u.id = $6 AND ur.role_id = $7
                RETURNING u.id, u.first_name, u.last_name, u.email, u.school_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at
                "#,
            )
            .bind(&first_name)
            .bind(&last_name)
            .bind(email.as_str())
            .bind(date_of_birth)
            .bind(&grade_level)
            .bind(id)
            .bind(student_role_id)
            .fetch_one(db)
            .await
        }
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Student with email {} already exists",
                    email
                ));
            }
            AppError::database(anyhow::Error::from(e))
        })?;

        Ok(updated_student)
    }

    #[instrument(skip(db))]
    pub async fn delete_student_no_school_filter(db: &PgPool, id: Uuid) -> Result<(), AppError> {
        let student_role_id = system_roles::STUDENT;

        // Check if user exists and is a student
        let student_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users u
                INNER JOIN user_roles ur ON ur.user_id = u.id
                WHERE u.id = $1 AND ur.role_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(student_role_id)
        .fetch_one(db)
        .await
        .context("Failed to check student existence")
        .map_err(AppError::database)?;

        if !student_exists {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        // Delete role assignment first
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(id)
            .execute(db)
            .await
            .context("Failed to delete student role assignment")
            .map_err(AppError::database)?;

        // Delete the user
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .context("Failed to delete student")
            .map_err(AppError::database)?;

        Ok(())
    }
}
