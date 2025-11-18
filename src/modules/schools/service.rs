use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::modules::users::model::{
    CreateSchoolDto, PaginatedSchoolsResponse, School, SchoolFilterParams,
};
use crate::utils::errors::AppError;
use crate::utils::pagination::PaginationMeta;

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
    pub async fn get_all_schools(
        db: &PgPool,
        filters: SchoolFilterParams,
    ) -> Result<PaginatedSchoolsResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let mut count_query = String::from("SELECT COUNT(*) FROM schools WHERE 1=1");
        let mut where_clause = String::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(name) = &filters.name {
            params.push(format!("%{}%", name));
            where_clause.push_str(&format!(" AND name ILIKE ${}", params.len()));
        }

        if let Some(address) = &filters.address {
            params.push(format!("%{}%", address));
            where_clause.push_str(&format!(" AND address ILIKE ${}", params.len()));
        }

        count_query.push_str(&where_clause);

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await?;

        let mut data_query = String::from("SELECT id, name, address FROM schools WHERE 1=1");
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, School>(&data_query);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let schools = data_sql.fetch_all(db).await?;

        let has_more = offset + limit < total;

        Ok(PaginatedSchoolsResponse {
            data: schools,
            meta: PaginationMeta {
                total,
                limit,
                offset,
                has_more,
            },
        })
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
