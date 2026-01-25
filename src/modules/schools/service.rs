use sqlx::PgPool;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use chalkbyte_cache::{RedisCache, invalidate, keys};
use chalkbyte_core::{AppError, PaginationMeta};

#[cfg(feature = "observability")]
use chalkbyte_observability::metrics;
use crate::modules::users::model::{
    CreateSchoolDto, PaginatedBasicUsersResponse, PaginatedSchoolsResponse, School,
    SchoolFilterParams, SchoolFullInfo, User, UserFilterParams, system_roles,
};

pub struct SchoolService;

impl SchoolService {
    #[instrument(skip(db, cache, dto), fields(school.name = %dto.name, db.operation = "INSERT", db.table = "schools"))]
    pub async fn create_school(
        db: &PgPool,
        cache: Option<&RedisCache>,
        dto: CreateSchoolDto,
    ) -> Result<School, AppError> {
        debug!(school.name = %dto.name, school.address = ?dto.address, "Creating new school");

        #[cfg(feature = "observability")]
        metrics::track_school_created();

        let school = sqlx::query_as::<_, School>(
            "INSERT INTO schools (name, address) VALUES ($1, $2)
             RETURNING id, name, address, created_at, updated_at",
        )
        .bind(&dto.name)
        .bind(&dto.address)
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                warn!(school.name = %dto.name, "Attempted to create school with existing name");
                return AppError::bad_request(anyhow::anyhow!("School name already exists"));
            }
            error!(error = %e, school.name = %dto.name, "Database error creating school");
            AppError::from(e)
        })?;

        // Invalidate list caches (new school should appear in lists)
        invalidate::school(cache, Some(school.id.into())).await;

        info!(
            school.id = %school.id,
            school.name = %school.name,
            "School created successfully"
        );

        Ok(school)
    }

    #[instrument(skip(db, filters), fields(db.operation = "SELECT", db.table = "schools"))]
    pub async fn get_all_schools(
        db: &PgPool,
        filters: SchoolFilterParams,
    ) -> Result<PaginatedSchoolsResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        debug!(
            limit = %limit,
            offset = %offset,
            filter.name = ?filters.name,
            filter.address = ?filters.address,
            "Fetching schools with pagination"
        );

        let mut count_query = String::from("SELECT COUNT(*) FROM schools WHERE 1=1");
        let mut where_clause = String::new();
        let mut params = Vec::new();

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
        let total = count_sql.fetch_one(db).await.map_err(|e| {
            error!(error = %e, "Database error counting schools");
            AppError::from(e)
        })?;

        let mut data_query =
            String::from("SELECT id, name, address, created_at, updated_at FROM schools WHERE 1=1");
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, School>(&data_query);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let schools = data_sql.fetch_all(db).await.map_err(|e| {
            error!(error = %e, "Database error fetching schools");
            AppError::from(e)
        })?;

        let has_more = offset + limit < total;

        debug!(
            total = %total,
            returned = %schools.len(),
            has_more = %has_more,
            "Schools fetched successfully"
        );

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

    #[instrument(skip(db, cache), fields(school.id = %school_id, db.operation = "SELECT", db.table = "schools"))]
    pub async fn get_school_by_id(
        db: &PgPool,
        cache: Option<&RedisCache>,
        school_id: Uuid,
    ) -> Result<School, AppError> {
        let cache_key = keys::schools::by_id(school_id);

        // Try cache first
        if let Some(cache) = cache
            && let Some(school) = cache.get::<School>(&cache_key).await
        {
            debug!(school.id = %school_id, "School found in cache");
            return Ok(school);
        }

        debug!("Fetching school by ID from database");

        let school = sqlx::query_as::<_, School>(
            "SELECT id, name, address, created_at, updated_at FROM schools WHERE id = $1",
        )
        .bind(school_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error fetching school");
            AppError::from(e)
        })?
        .ok_or_else(|| {
            debug!(school.id = %school_id, "School not found");
            AppError::not_found(anyhow::anyhow!("School not found"))
        })?;

        // Cache the result
        if let Some(cache) = cache
            && let Err(e) = cache.set(&cache_key, &school).await
        {
            warn!(error = %e, "Failed to cache school");
        }

        debug!(school.name = %school.name, "School found");

        Ok(school)
    }

    #[instrument(skip(db, cache), fields(school.id = %school_id, db.operation = "DELETE", db.table = "schools"))]
    pub async fn delete_school(
        db: &PgPool,
        cache: Option<&RedisCache>,
        school_id: Uuid,
    ) -> Result<(), AppError> {
        debug!("Deleting school");

        let result = sqlx::query("DELETE FROM schools WHERE id = $1")
            .bind(school_id)
            .execute(db)
            .await
            .map_err(|e| {
                error!(school.id = %school_id, error = %e, "Database error deleting school");
                AppError::from(e)
            })?;

        if result.rows_affected() == 0 {
            debug!(school.id = %school_id, "School not found for deletion");
            return Err(AppError::not_found(anyhow::anyhow!("School not found")));
        }

        // Invalidate cache
        invalidate::school(cache, Some(school_id)).await;

        info!(school.id = %school_id, "School deleted successfully");

        Ok(())
    }

    #[instrument(skip(db, filters), fields(school.id = %school_id, db.operation = "SELECT", db.table = "users"))]
    pub async fn get_school_students(
        db: &PgPool,
        school_id: Uuid,
        filters: UserFilterParams,
    ) -> Result<PaginatedBasicUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        debug!(
            limit = %limit,
            offset = %offset,
            filter.first_name = ?filters.first_name,
            filter.last_name = ?filters.last_name,
            "Fetching students for school"
        );

        let student_role_id = system_roles::STUDENT;
        let mut count_query = String::from(
            "SELECT COUNT(*) FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        );
        let mut where_clause = String::new();
        let mut params = Vec::new();

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

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query)
            .bind(school_id)
            .bind(student_role_id);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await.map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error counting students");
            AppError::from(e)
        })?;

        let mut data_query = String::from(
            "SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.level_id, u.branch_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        );
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, User>(&data_query)
            .bind(school_id)
            .bind(student_role_id);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let users = data_sql.fetch_all(db).await.map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error fetching students");
            AppError::from(e)
        })?;

        let has_more = offset + limit < total;

        debug!(
            school.id = %school_id,
            total = %total,
            returned = %users.len(),
            "Students fetched successfully"
        );

        Ok(PaginatedBasicUsersResponse {
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

    #[instrument(skip(db, filters), fields(school.id = %school_id, db.operation = "SELECT", db.table = "users"))]
    pub async fn get_school_admins(
        db: &PgPool,
        school_id: Uuid,
        filters: UserFilterParams,
    ) -> Result<PaginatedBasicUsersResponse, AppError> {
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        debug!(
            limit = %limit,
            offset = %offset,
            "Fetching admins for school"
        );

        let admin_role_id = system_roles::ADMIN;
        let mut count_query = String::from(
            "SELECT COUNT(*) FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        );
        let mut where_clause = String::new();
        let mut params = Vec::new();

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

        let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query)
            .bind(school_id)
            .bind(admin_role_id);
        for param in &params {
            count_sql = count_sql.bind(param);
        }
        let total = count_sql.fetch_one(db).await.map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error counting admins");
            AppError::from(e)
        })?;

        let mut data_query = String::from(
            "SELECT u.id, u.first_name, u.last_name, u.email, u.school_id, u.level_id, u.branch_id, u.date_of_birth, u.grade_level, u.created_at, u.updated_at FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        );
        data_query.push_str(&where_clause);
        data_query.push_str(" ORDER BY created_at DESC");
        data_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut data_sql = sqlx::query_as::<_, User>(&data_query)
            .bind(school_id)
            .bind(admin_role_id);
        for param in params {
            data_sql = data_sql.bind(param);
        }
        let users = data_sql.fetch_all(db).await.map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error fetching admins");
            AppError::from(e)
        })?;

        let has_more = offset + limit < total;

        debug!(
            school.id = %school_id,
            total = %total,
            returned = %users.len(),
            "Admins fetched successfully"
        );

        Ok(PaginatedBasicUsersResponse {
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

    #[instrument(skip(db), fields(school.id = %school_id, db.operation = "SELECT", db.table = "schools,users"))]
    pub async fn get_school_full_info(
        db: &PgPool,
        school_id: Uuid,
    ) -> Result<SchoolFullInfo, AppError> {
        debug!("Fetching full school information with statistics");

        let school = sqlx::query_as::<_, School>(
            "SELECT id, name, address, created_at, updated_at FROM schools WHERE id = $1",
        )
        .bind(school_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error fetching school");
            AppError::from(e)
        })?
        .ok_or_else(|| {
            debug!(school.id = %school_id, "School not found");
            AppError::not_found(anyhow::anyhow!("School not found"))
        })?;

        debug!(school.name = %school.name, "School found, fetching statistics");

        let total_students = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        )
        .bind(school_id)
        .bind(system_roles::STUDENT)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error counting students");
            AppError::from(e)
        })?;

        let total_teachers = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        )
        .bind(school_id)
        .bind(system_roles::TEACHER)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error counting teachers");
            AppError::from(e)
        })?;

        let total_admins = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users u INNER JOIN user_roles ur ON ur.user_id = u.id WHERE u.school_id = $1 AND ur.role_id = $2",
        )
        .bind(school_id)
        .bind(system_roles::ADMIN)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!(school.id = %school_id, error = %e, "Database error counting admins");
            AppError::from(e)
        })?;

        info!(
            school.id = %school_id,
            school.name = %school.name,
            total_students = %total_students,
            total_teachers = %total_teachers,
            total_admins = %total_admins,
            "School full info retrieved successfully"
        );

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
