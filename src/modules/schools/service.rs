use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::modules::users::model::{CreateSchoolDto, School};
use crate::utils::errors::AppError;

pub struct SchoolService;

impl SchoolService {
    #[instrument]
    pub async fn create_school(db: &PgPool, dto: CreateSchoolDto) -> Result<School, AppError> {
        let school = sqlx::query_as::<_, School>(
            "INSERT INTO schools (name, address) VALUES ($1, $2) 
             RETURNING id, name, address",
        )
        .bind(&dto.name)
        .bind(&dto.address)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!("School name already exists"));
            }
            AppError::from(e)
        })?;

        Ok(school)
    }

    #[instrument]
    pub async fn get_all_schools(db: &PgPool) -> Result<Vec<School>, AppError> {
        let schools = sqlx::query_as::<_, School>(
            "SELECT id, name, address FROM schools ORDER BY created_at DESC",
        )
        .fetch_all(db)
        .await?;

        Ok(schools)
    }

    #[instrument]
    pub async fn get_school_by_id(db: &PgPool, school_id: Uuid) -> Result<School, AppError> {
        let school =
            sqlx::query_as::<_, School>("SELECT id, name, address FROM schools WHERE id = $1")
                .bind(school_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| AppError::not_found(anyhow::anyhow!("School not found")))?;

        Ok(school)
    }

    #[instrument]
    pub async fn delete_school(db: &PgPool, school_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM schools WHERE id = $1")
            .bind(school_id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("School not found")));
        }

        Ok(())
    }
}
