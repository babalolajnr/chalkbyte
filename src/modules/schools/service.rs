use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::metrics;
use crate::modules::users::model::{
    CreateSchoolDto, PaginatedSchoolsResponse, PaginatedUsersResponse, School, SchoolFilterParams,
    SchoolFullInfo, User, UserFilterParams,
};
use crate::utils::errors::AppError;
use crate::utils::pagination::PaginationMeta;

pub struct SchoolService;

impl SchoolService {
    #[instrument]
    pub async fn create_school(db: &PgPool, dto: CreateSchoolDto) -> Result<School, AppError> {
        metrics::track_school_created();

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
                offset: Some(offset),
                page: None,
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

    #[instrument]
    pub async fn get_school_students(
        db: &PgPool,
        school_id: Uuid,
        filters: UserFilterParams,
    ) -> Result<PaginatedUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let mut count_query =
            String::from("SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'student'");
        let mut where_clause = String::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(first_name) = &filters.first_name {
            params.push(format!("%{}%", first_name));
            where_clause.push_str(&format!(" AND first_name ILIKE ${}", params.len() + 1));
        }

        if let Some(last_name) = &filters.last_name {
            params.push(format!("%{}%", last_name));
            where_clause.push_str(&format!(" AND last_name ILIKE ${}", params.len() + 1));
        }

        if let Some(email) = &filters.email {
            params.push(format!("%{}%", email));
            where_clause.push_str(&format!(" AND email ILIKE ${}", params.len() + 1));
        }

        count_query.push_str(&where_clause);

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query).bind(school_id);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await?;

        let mut data_query = String::from(
            "SELECT id, first_name, last_name, email, role, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE school_id = $1 AND role = 'student'",
        );
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, User>(&data_query).bind(school_id);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let users = data_sql.fetch_all(db).await?;

        let has_more = offset + limit < total;

        Ok(PaginatedUsersResponse {
            data: users,
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
    pub async fn get_school_admins(
        db: &PgPool,
        school_id: Uuid,
        filters: UserFilterParams,
    ) -> Result<PaginatedUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let mut count_query =
            String::from("SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'admin'");
        let mut where_clause = String::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(first_name) = &filters.first_name {
            params.push(format!("%{}%", first_name));
            where_clause.push_str(&format!(" AND first_name ILIKE ${}", params.len() + 1));
        }

        if let Some(last_name) = &filters.last_name {
            params.push(format!("%{}%", last_name));
            where_clause.push_str(&format!(" AND last_name ILIKE ${}", params.len() + 1));
        }

        if let Some(email) = &filters.email {
            params.push(format!("%{}%", email));
            where_clause.push_str(&format!(" AND email ILIKE ${}", params.len() + 1));
        }

        count_query.push_str(&where_clause);

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query).bind(school_id);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await?;

        let mut data_query = String::from(
            "SELECT id, first_name, last_name, email, role, school_id FROM users WHERE school_id = $1 AND role = 'admin'",
        );
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, User>(&data_query).bind(school_id);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let users = data_sql.fetch_all(db).await?;

        let has_more = offset + limit < total;

        Ok(PaginatedUsersResponse {
            data: users,
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
    pub async fn get_school_full_info(
        db: &PgPool,
        school_id: Uuid,
    ) -> Result<SchoolFullInfo, AppError> {
        let school =
            sqlx::query_as::<_, School>("SELECT id, name, address FROM schools WHERE id = $1")
                .bind(school_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| AppError::not_found(anyhow::anyhow!("School not found")))?;

        let total_students = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'student'",
        )
        .bind(school_id)
        .fetch_one(db)
        .await?;

        let total_teachers = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'teacher'",
        )
        .bind(school_id)
        .fetch_one(db)
        .await?;

        let total_admins = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'admin'",
        )
        .bind(school_id)
        .fetch_one(db)
        .await?;

        Ok(SchoolFullInfo {
            id: school.id,
            name: school.name,
            address: school.address,
            total_students,
            total_teachers,
            total_admins,
        })
    }
}
