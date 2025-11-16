use crate::{
    modules::students::model::{CreateStudentDto, Student, UpdateStudentDto},
    utils::{errors::AppError, password::hash_password},
};
use anyhow::Context;
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

        let student = sqlx::query_as!(
            Student,
            r#"
            INSERT INTO users (first_name, last_name, email, password, role, school_id)
            VALUES ($1, $2, $3, $4, 'student', $5)
            RETURNING id, first_name, last_name, email, school_id, created_at, updated_at
            "#,
            dto.first_name,
            dto.last_name,
            dto.email,
            hashed_password,
            school_id
        )
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::bad_request(anyhow::anyhow!(
                        "Student with email {} already exists",
                        dto.email
                    ));
                }
            }
            AppError::database(anyhow::Error::from(e))
        })?;

        Ok(student)
    }

    #[instrument(skip(db))]
    pub async fn get_students_by_school(
        db: &PgPool,
        school_id: Uuid,
    ) -> Result<Vec<Student>, AppError> {
        let students = sqlx::query_as!(
            Student,
            r#"
            SELECT id, first_name, last_name, email, school_id, created_at, updated_at
            FROM users
            WHERE school_id = $1 AND role = 'student'
            ORDER BY last_name, first_name
            "#,
            school_id
        )
        .fetch_all(db)
        .await
        .context("Failed to fetch students by school")
        .map_err(AppError::database)?;

        Ok(students)
    }

    #[instrument(skip(db))]
    pub async fn get_student_by_id(
        db: &PgPool,
        id: Uuid,
        school_id: Uuid,
    ) -> Result<Student, AppError> {
        let student = sqlx::query_as!(
            Student,
            r#"
            SELECT id, first_name, last_name, email, school_id, created_at, updated_at
            FROM users
            WHERE id = $1 AND school_id = $2 AND role = 'student'
            "#,
            id,
            school_id
        )
        .fetch_optional(db)
        .await
        .context("Failed to fetch student by ID")
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found(anyhow::anyhow!("Student not found")))?;

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
        let email = dto.email.unwrap_or(existing.email);

        let updated_student = if let Some(password) = dto.password {
            let hashed_password = hash_password(&password)?;
            sqlx::query_as!(
                Student,
                r#"
                UPDATE users
                SET first_name = $1, last_name = $2, email = $3, password = $4, updated_at = NOW()
                WHERE id = $5 AND school_id = $6 AND role = 'student'
                RETURNING id, first_name, last_name, email, school_id, created_at, updated_at
                "#,
                first_name,
                last_name,
                email,
                hashed_password,
                id,
                school_id
            )
            .fetch_one(db)
            .await
        } else {
            sqlx::query_as!(
                Student,
                r#"
                UPDATE users
                SET first_name = $1, last_name = $2, email = $3, updated_at = NOW()
                WHERE id = $4 AND school_id = $5 AND role = 'student'
                RETURNING id, first_name, last_name, email, school_id, created_at, updated_at
                "#,
                first_name,
                last_name,
                email,
                id,
                school_id
            )
            .fetch_one(db)
            .await
        }
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::bad_request(anyhow::anyhow!(
                        "Student with email {} already exists",
                        email
                    ));
                }
            }
            AppError::database(anyhow::Error::from(e))
        })?;

        Ok(updated_student)
    }

    #[instrument(skip(db))]
    pub async fn delete_student(db: &PgPool, id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1 AND school_id = $2 AND role = 'student'
            "#,
            id,
            school_id
        )
        .execute(db)
        .await
        .context("Failed to delete student")
        .map_err(AppError::database)?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }
}
