use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use chalkbyte_core::{AppError, PaginationMeta};

use crate::modules::users::model::system_roles;

use super::model::{
    AssignStudentsToBranchDto, Branch, BranchFilterParams, BranchWithStats, BulkAssignResponse,
    CreateBranchDto, MoveStudentToBranchDto, PaginatedBranchesResponse, UpdateBranchDto,
};

pub struct BranchService;

impl BranchService {
    #[instrument(skip(db))]
    pub async fn create_branch(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
        dto: CreateBranchDto,
    ) -> Result<Branch, AppError> {
        let level = sqlx::query!(
            r#"SELECT id, school_id FROM levels WHERE id = $1"#,
            level_id
        )
        .fetch_optional(db)
        .await?;

        let level = level.ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        if level.school_id != school_id {
            return Err(AppError::forbidden(
                "Cannot create branch for level in another school".to_string(),
            ));
        }

        let branch = sqlx::query_as!(
            Branch,
            r#"
            INSERT INTO branches (name, description, level_id)
            VALUES ($1, $2, $3)
            RETURNING id, name, description, level_id, created_at, updated_at
            "#,
            dto.name,
            dto.description,
            level_id
        )
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Branch with this name already exists for this level"
                ));
            }
            AppError::from(e)
        })?;

        Ok(branch)
    }

    #[instrument(skip(db))]
    pub async fn get_branches_by_level(
        db: &PgPool,
        level_id: Uuid,
        school_id: Uuid,
        filters: BranchFilterParams,
    ) -> Result<PaginatedBranchesResponse, AppError> {
        let level = sqlx::query!(
            r#"SELECT id, school_id FROM levels WHERE id = $1"#,
            level_id
        )
        .fetch_optional(db)
        .await?;

        let level = level.ok_or_else(|| AppError::not_found(anyhow::anyhow!("Level not found")))?;

        if level.school_id != school_id {
            return Err(AppError::forbidden(
                "Cannot access branches for level in another school".to_string(),
            ));
        }

        let page = filters.pagination.page();
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let student_role_id = system_roles::STUDENT;
        let branches = if let Some(name) = &filters.name {
            sqlx::query_as::<_, BranchWithStats>(
                r#"
                SELECT
                    b.id,
                    b.name,
                    b.description,
                    b.level_id,
                    b.created_at,
                    b.updated_at,
                    COUNT(u.id)::bigint as student_count
                FROM branches b
                LEFT JOIN users u ON u.branch_id = b.id
                LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $5
                WHERE b.level_id = $1 AND b.name ILIKE $2 AND (u.id IS NULL OR ur.role_id IS NOT NULL)
                GROUP BY b.id
                ORDER BY b.created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(level_id)
            .bind(format!("%{}%", name))
            .bind(limit)
            .bind(offset)
            .bind(student_role_id)
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as::<_, BranchWithStats>(
                r#"
                SELECT
                    b.id,
                    b.name,
                    b.description,
                    b.level_id,
                    b.created_at,
                    b.updated_at,
                    COUNT(DISTINCT CASE WHEN ur.role_id IS NOT NULL THEN u.id END)::bigint as student_count
                FROM branches b
                LEFT JOIN users u ON u.branch_id = b.id
                LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $4
                WHERE b.level_id = $1
                GROUP BY b.id
                ORDER BY b.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(level_id)
            .bind(limit)
            .bind(offset)
            .bind(student_role_id)
            .fetch_all(db)
            .await?
        };

        let total_query = if let Some(name) = &filters.name {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM branches WHERE level_id = $1 AND name ILIKE $2",
                level_id,
                format!("%{}%", name)
            )
            .fetch_one(db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM branches WHERE level_id = $1",
                level_id
            )
            .fetch_one(db)
            .await?
        };

        let total = total_query.unwrap_or(0);

        Ok(PaginatedBranchesResponse {
            data: branches,
            meta: PaginationMeta {
                total,
                limit,
                offset: Some(offset),
                page,
                has_more: offset + limit < total,
            },
        })
    }

    #[instrument(skip(db))]
    pub async fn get_branch_by_id(
        db: &PgPool,
        id: Uuid,
        school_id: Uuid,
    ) -> Result<BranchWithStats, AppError> {
        let student_role_id = system_roles::STUDENT;
        let branch = sqlx::query_as::<_, BranchWithStats>(
            r#"
            SELECT
                b.id,
                b.name,
                b.description,
                b.level_id,
                b.created_at,
                b.updated_at,
                COUNT(DISTINCT CASE WHEN ur.role_id IS NOT NULL THEN u.id END)::bigint as student_count
            FROM branches b
            INNER JOIN levels l ON l.id = b.level_id
            LEFT JOIN users u ON u.branch_id = b.id
            LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $3
            WHERE b.id = $1 AND l.school_id = $2
            GROUP BY b.id
            "#,
        )
        .bind(id)
        .bind(school_id)
        .bind(student_role_id)
        .fetch_optional(db)
        .await?;

        branch.ok_or_else(|| AppError::not_found(anyhow::anyhow!("Branch not found")))
    }

    #[instrument(skip(db))]
    pub async fn update_branch(
        db: &PgPool,
        id: Uuid,
        school_id: Uuid,
        dto: UpdateBranchDto,
    ) -> Result<Branch, AppError> {
        let existing = sqlx::query!(
            r#"
            SELECT b.id
            FROM branches b
            INNER JOIN levels l ON l.id = b.level_id
            WHERE b.id = $1 AND l.school_id = $2
            "#,
            id,
            school_id
        )
        .fetch_optional(db)
        .await?;

        if existing.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        let mut query = String::from("UPDATE branches SET updated_at = NOW()");
        let mut param_count = 1;

        if dto.name.is_some() {
            param_count += 1;
            query.push_str(&format!(", name = ${}", param_count));
        }

        if dto.description.is_some() {
            param_count += 1;
            query.push_str(&format!(", description = ${}", param_count));
        }

        query.push_str(
            " WHERE id = $1 RETURNING id, name, description, level_id, created_at, updated_at",
        );

        let mut query_builder = sqlx::query_as::<_, Branch>(&query).bind(id);

        if let Some(name) = dto.name {
            query_builder = query_builder.bind(name);
        }

        if let Some(description) = dto.description {
            query_builder = query_builder.bind(description);
        }

        let branch = query_builder.fetch_one(db).await.map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Branch with this name already exists for this level"
                ));
            }
            AppError::from(e)
        })?;

        Ok(branch)
    }

    #[instrument(skip(db))]
    pub async fn delete_branch(db: &PgPool, id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM branches
            WHERE id = $1 AND level_id IN (
                SELECT id FROM levels WHERE school_id = $2
            )
            "#,
            id,
            school_id
        )
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        Ok(())
    }

    #[instrument(skip(db))]
    pub async fn assign_students_to_branch(
        db: &PgPool,
        branch_id: Uuid,
        school_id: Uuid,
        dto: AssignStudentsToBranchDto,
    ) -> Result<BulkAssignResponse, AppError> {
        let branch = sqlx::query!(
            r#"
            SELECT b.id
            FROM branches b
            INNER JOIN levels l ON l.id = b.level_id
            WHERE b.id = $1 AND l.school_id = $2
            "#,
            branch_id,
            school_id
        )
        .fetch_optional(db)
        .await?;

        if branch.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        let mut assigned_count = 0;
        let mut failed_ids = Vec::new();

        let student_role_id = system_roles::STUDENT;
        for student_id in dto.student_ids {
            // Check if user has student role
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
                r#"
                UPDATE users
                SET branch_id = $1, updated_at = NOW()
                WHERE id = $2 AND school_id = $3
                "#,
            )
            .bind(branch_id)
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

    #[instrument(skip(db))]
    pub async fn move_student_to_branch(
        db: &PgPool,
        student_id: Uuid,
        school_id: Uuid,
        dto: MoveStudentToBranchDto,
    ) -> Result<(), AppError> {
        if let Some(branch_id) = dto.branch_id {
            let branch = sqlx::query!(
                r#"
                SELECT b.id
                FROM branches b
                INNER JOIN levels l ON l.id = b.level_id
                WHERE b.id = $1 AND l.school_id = $2
                "#,
                branch_id,
                school_id
            )
            .fetch_optional(db)
            .await?;

            if branch.is_none() {
                return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
            }
        }

        // Check if user has student role
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

        let result = sqlx::query(
            r#"
            UPDATE users
            SET branch_id = $1, updated_at = NOW()
            WHERE id = $2 AND school_id = $3
            "#,
        )
        .bind(dto.branch_id)
        .bind(student_id)
        .bind(school_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }

    #[instrument(skip(db))]
    pub async fn get_students_in_branch(
        db: &PgPool,
        branch_id: Uuid,
        school_id: Uuid,
    ) -> Result<Vec<crate::modules::users::model::User>, AppError> {
        let branch = sqlx::query!(
            r#"
            SELECT b.id
            FROM branches b
            INNER JOIN levels l ON l.id = b.level_id
            WHERE b.id = $1 AND l.school_id = $2
            "#,
            branch_id,
            school_id
        )
        .fetch_optional(db)
        .await?;

        if branch.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        let student_role_id = system_roles::STUDENT;
        let students = sqlx::query_as::<_, crate::modules::users::model::User>(
            r#"
            SELECT
                u.id,
                u.first_name,
                u.last_name,
                u.email,
                u.school_id,
                u.level_id,
                u.branch_id,
                u.date_of_birth,
                u.grade_level,
                u.created_at,
                u.updated_at
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.branch_id = $1 AND ur.role_id = $2
            ORDER BY u.last_name, u.first_name
            "#,
        )
        .bind(branch_id)
        .bind(student_role_id)
        .fetch_all(db)
        .await?;

        Ok(students)
    }

    #[instrument(skip(db))]
    pub async fn remove_student_from_branch(
        db: &PgPool,
        student_id: Uuid,
        school_id: Uuid,
    ) -> Result<(), AppError> {
        let student_role_id = system_roles::STUDENT;
        let result = sqlx::query(
            r#"
            UPDATE users
            SET branch_id = NULL, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
            AND EXISTS (
                SELECT 1 FROM user_roles ur
                WHERE ur.user_id = $1 AND ur.role_id = $3
            )
            "#,
        )
        .bind(student_id)
        .bind(school_id)
        .bind(student_role_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }

    // =====================================================
    // No school filter variants for system admin operations
    // =====================================================

    #[instrument(skip(db))]
    pub async fn create_branch_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
        dto: CreateBranchDto,
    ) -> Result<Branch, AppError> {
        // Verify level exists (no school check)
        let level = sqlx::query!(r#"SELECT id FROM levels WHERE id = $1"#, level_id)
            .fetch_optional(db)
            .await?;

        if level.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let branch = sqlx::query_as!(
            Branch,
            r#"
            INSERT INTO branches (name, description, level_id)
            VALUES ($1, $2, $3)
            RETURNING id, name, description, level_id, created_at, updated_at
            "#,
            dto.name,
            dto.description,
            level_id
        )
        .fetch_one(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Branch with this name already exists for this level"
                ));
            }
            AppError::from(e)
        })?;

        Ok(branch)
    }

    #[instrument(skip(db))]
    pub async fn get_branches_by_level_no_school_filter(
        db: &PgPool,
        level_id: Uuid,
        filters: BranchFilterParams,
    ) -> Result<PaginatedBranchesResponse, AppError> {
        // Verify level exists (no school check)
        let level = sqlx::query!(r#"SELECT id FROM levels WHERE id = $1"#, level_id)
            .fetch_optional(db)
            .await?;

        if level.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Level not found")));
        }

        let page = filters.pagination.page();
        let limit = filters.pagination.limit();
        let offset = filters.pagination.offset();

        let student_role_id = system_roles::STUDENT;
        let branches = if let Some(name) = &filters.name {
            sqlx::query_as::<_, BranchWithStats>(
                r#"
                SELECT
                    b.id,
                    b.name,
                    b.description,
                    b.level_id,
                    b.created_at,
                    b.updated_at,
                    COUNT(u.id)::bigint as student_count
                FROM branches b
                LEFT JOIN users u ON u.branch_id = b.id
                LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $5
                WHERE b.level_id = $1 AND b.name ILIKE $2 AND (u.id IS NULL OR ur.role_id IS NOT NULL)
                GROUP BY b.id
                ORDER BY b.created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(level_id)
            .bind(format!("%{}%", name))
            .bind(limit)
            .bind(offset)
            .bind(student_role_id)
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as::<_, BranchWithStats>(
                r#"
                SELECT
                    b.id,
                    b.name,
                    b.description,
                    b.level_id,
                    b.created_at,
                    b.updated_at,
                    COUNT(DISTINCT CASE WHEN ur.role_id IS NOT NULL THEN u.id END)::bigint as student_count
                FROM branches b
                LEFT JOIN users u ON u.branch_id = b.id
                LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $4
                WHERE b.level_id = $1
                GROUP BY b.id
                ORDER BY b.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(level_id)
            .bind(limit)
            .bind(offset)
            .bind(student_role_id)
            .fetch_all(db)
            .await?
        };

        let total_query = if let Some(name) = &filters.name {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM branches WHERE level_id = $1 AND name ILIKE $2",
                level_id,
                format!("%{}%", name)
            )
            .fetch_one(db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM branches WHERE level_id = $1",
                level_id
            )
            .fetch_one(db)
            .await?
        };

        let total = total_query.unwrap_or(0);

        Ok(PaginatedBranchesResponse {
            data: branches,
            meta: PaginationMeta {
                total,
                limit,
                offset: Some(offset),
                page,
                has_more: offset + limit < total,
            },
        })
    }

    #[instrument(skip(db))]
    pub async fn get_branch_by_id_no_school_filter(
        db: &PgPool,
        id: Uuid,
    ) -> Result<BranchWithStats, AppError> {
        let student_role_id = system_roles::STUDENT;
        let branch = sqlx::query_as::<_, BranchWithStats>(
            r#"
            SELECT
                b.id,
                b.name,
                b.description,
                b.level_id,
                b.created_at,
                b.updated_at,
                COUNT(DISTINCT CASE WHEN ur.role_id IS NOT NULL THEN u.id END)::bigint as student_count
            FROM branches b
            LEFT JOIN users u ON u.branch_id = b.id
            LEFT JOIN user_roles ur ON ur.user_id = u.id AND ur.role_id = $2
            WHERE b.id = $1
            GROUP BY b.id
            "#,
        )
        .bind(id)
        .bind(student_role_id)
        .fetch_optional(db)
        .await?;

        branch.ok_or_else(|| AppError::not_found(anyhow::anyhow!("Branch not found")))
    }

    #[instrument(skip(db))]
    pub async fn update_branch_no_school_filter(
        db: &PgPool,
        id: Uuid,
        dto: UpdateBranchDto,
    ) -> Result<Branch, AppError> {
        let existing = sqlx::query!(r#"SELECT id FROM branches WHERE id = $1"#, id)
            .fetch_optional(db)
            .await?;

        if existing.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        let mut query = String::from("UPDATE branches SET updated_at = NOW()");
        let mut param_count = 1;

        if dto.name.is_some() {
            param_count += 1;
            query.push_str(&format!(", name = ${}", param_count));
        }

        if dto.description.is_some() {
            param_count += 1;
            query.push_str(&format!(", description = ${}", param_count));
        }

        query.push_str(
            " WHERE id = $1 RETURNING id, name, description, level_id, created_at, updated_at",
        );

        let mut query_builder = sqlx::query_as::<_, Branch>(&query).bind(id);

        if let Some(name) = dto.name {
            query_builder = query_builder.bind(name);
        }

        if let Some(description) = dto.description {
            query_builder = query_builder.bind(description);
        }

        let branch = query_builder.fetch_one(db).await.map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e
                && db_err.is_unique_violation()
            {
                return AppError::bad_request(anyhow::anyhow!(
                    "Branch with this name already exists for this level"
                ));
            }
            AppError::from(e)
        })?;

        Ok(branch)
    }

    #[instrument(skip(db))]
    pub async fn delete_branch_no_school_filter(db: &PgPool, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(r#"DELETE FROM branches WHERE id = $1"#, id)
            .execute(db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        Ok(())
    }

    #[instrument(skip(db))]
    pub async fn assign_students_to_branch_no_school_filter(
        db: &PgPool,
        branch_id: Uuid,
        dto: AssignStudentsToBranchDto,
    ) -> Result<BulkAssignResponse, AppError> {
        let branch = sqlx::query!(r#"SELECT id FROM branches WHERE id = $1"#, branch_id)
            .fetch_optional(db)
            .await?;

        if branch.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
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

            let result = sqlx::query(
                r#"
                UPDATE users
                SET branch_id = $1, updated_at = NOW()
                WHERE id = $2
                "#,
            )
            .bind(branch_id)
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

    #[instrument(skip(db))]
    pub async fn get_students_in_branch_no_school_filter(
        db: &PgPool,
        branch_id: Uuid,
    ) -> Result<Vec<crate::modules::users::model::User>, AppError> {
        let branch = sqlx::query!(r#"SELECT id FROM branches WHERE id = $1"#, branch_id)
            .fetch_optional(db)
            .await?;

        if branch.is_none() {
            return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
        }

        let student_role_id = system_roles::STUDENT;
        let students = sqlx::query_as::<_, crate::modules::users::model::User>(
            r#"
            SELECT
                u.id,
                u.first_name,
                u.last_name,
                u.email,
                u.school_id,
                u.level_id,
                u.branch_id,
                u.date_of_birth,
                u.grade_level,
                u.created_at,
                u.updated_at
            FROM users u
            INNER JOIN user_roles ur ON ur.user_id = u.id
            WHERE u.branch_id = $1 AND ur.role_id = $2
            ORDER BY u.last_name, u.first_name
            "#,
        )
        .bind(branch_id)
        .bind(student_role_id)
        .fetch_all(db)
        .await?;

        Ok(students)
    }

    #[instrument(skip(db))]
    pub async fn move_student_to_branch_no_school_filter(
        db: &PgPool,
        student_id: Uuid,
        dto: MoveStudentToBranchDto,
    ) -> Result<(), AppError> {
        if let Some(branch_id) = dto.branch_id {
            let branch = sqlx::query!(r#"SELECT id FROM branches WHERE id = $1"#, branch_id)
                .fetch_optional(db)
                .await?;

            if branch.is_none() {
                return Err(AppError::not_found(anyhow::anyhow!("Branch not found")));
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

        let result = sqlx::query(
            r#"
            UPDATE users
            SET branch_id = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(dto.branch_id)
        .bind(student_id)
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(anyhow::anyhow!("Student not found")));
        }

        Ok(())
    }

    #[instrument(skip(db))]
    pub async fn remove_student_from_branch_no_school_filter(
        db: &PgPool,
        student_id: Uuid,
    ) -> Result<(), AppError> {
        let student_role_id = system_roles::STUDENT;
        let result = sqlx::query(
            r#"
            UPDATE users
            SET branch_id = NULL, updated_at = NOW()
            WHERE id = $1
            AND EXISTS (
                SELECT 1 FROM user_roles ur
                WHERE ur.user_id = $1 AND ur.role_id = $2
            )
            "#,
        )
        .bind(student_id)
        .bind(student_role_id)
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
    use crate::modules::branches::model::{
        AssignStudentsToBranchDto, BranchFilterParams, CreateBranchDto, MoveStudentToBranchDto,
        UpdateBranchDto,
    };
    use chalkbyte_core::PaginationParams;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn setup_test_data(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
        let school_id = sqlx::query_scalar!(
            r#"INSERT INTO schools (name, address) VALUES ($1, $2) RETURNING id"#,
            format!("Test School {}", Uuid::new_v4()),
            "Test Address"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let level_id = sqlx::query_scalar!(
            r#"INSERT INTO levels (name, school_id) VALUES ($1, $2) RETURNING id"#,
            format!("Test Level {}", Uuid::new_v4()),
            school_id
        )
        .fetch_one(pool)
        .await
        .unwrap();

        (school_id, level_id, Uuid::new_v4())
    }

    async fn create_student(pool: &PgPool, school_id: Uuid) -> Uuid {
        let user_id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (first_name, last_name, email, password, school_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            "Test",
            "Student",
            format!("student-{}@test.com", Uuid::new_v4()),
            "$2b$10$test",
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
    async fn test_create_branch_success(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: Some("Test Description".to_string()),
        };

        let result = BranchService::create_branch(&pool, level_id, school_id, dto).await;

        assert!(result.is_ok());
        let branch = result.unwrap();
        assert_eq!(branch.name, "Test Branch");
        assert_eq!(branch.description, Some("Test Description".to_string()));
        assert_eq!(branch.level_id, level_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_branch_level_not_found(pool: PgPool) {
        let (school_id, _, _) = setup_test_data(&pool).await;
        let non_existent_level_id = Uuid::new_v4();

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };

        let result =
            BranchService::create_branch(&pool, non_existent_level_id, school_id, dto).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_branch_wrong_school(pool: PgPool) {
        let (_school_id, level_id, _) = setup_test_data(&pool).await;
        let wrong_school_id = Uuid::new_v4();

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };

        let result = BranchService::create_branch(&pool, level_id, wrong_school_id, dto).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_duplicate_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto1 = CreateBranchDto {
            name: "Duplicate Branch".to_string(),
            description: None,
        };

        BranchService::create_branch(&pool, level_id, school_id, dto1)
            .await
            .unwrap();

        let dto2 = CreateBranchDto {
            name: "Duplicate Branch".to_string(),
            description: None,
        };

        let result = BranchService::create_branch(&pool, level_id, school_id, dto2).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_branches_by_level(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        for i in 1..=3 {
            let dto = CreateBranchDto {
                name: format!("Branch {}", i),
                description: None,
            };
            BranchService::create_branch(&pool, level_id, school_id, dto)
                .await
                .unwrap();
        }

        let filters = BranchFilterParams {
            name: None,
            pagination: PaginationParams {
                page: Some(1),
                limit: Some(10),
                offset: None,
            },
        };

        let result =
            BranchService::get_branches_by_level(&pool, level_id, school_id, filters).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 3);
        assert!(response.meta.total >= 3);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_branches_with_name_filter(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto1 = CreateBranchDto {
            name: "Science Branch".to_string(),
            description: None,
        };
        BranchService::create_branch(&pool, level_id, school_id, dto1)
            .await
            .unwrap();

        let dto2 = CreateBranchDto {
            name: "Arts Branch".to_string(),
            description: None,
        };
        BranchService::create_branch(&pool, level_id, school_id, dto2)
            .await
            .unwrap();

        let filters = BranchFilterParams {
            name: Some("Science".to_string()),
            pagination: PaginationParams {
                page: Some(1),
                limit: Some(10),
                offset: None,
            },
        };

        let result =
            BranchService::get_branches_by_level(&pool, level_id, school_id, filters).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].name, "Science Branch");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_branch_by_id(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let result = BranchService::get_branch_by_id(&pool, branch.id, school_id).await;

        assert!(result.is_ok());
        let fetched = result.unwrap();
        assert_eq!(fetched.id, branch.id);
        assert_eq!(fetched.name, "Test Branch");
        assert_eq!(fetched.student_count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_branch_by_id_not_found(pool: PgPool) {
        let (school_id, _, _) = setup_test_data(&pool).await;
        let non_existent_id = Uuid::new_v4();

        let result = BranchService::get_branch_by_id(&pool, non_existent_id, school_id).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Original Name".to_string(),
            description: Some("Original Description".to_string()),
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let update_dto = UpdateBranchDto {
            name: Some("Updated Name".to_string()),
            description: Some("Updated Description".to_string()),
        };

        let result = BranchService::update_branch(&pool, branch.id, school_id, update_dto).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("Updated Description".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_branch_partial(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Original Name".to_string(),
            description: Some("Original Description".to_string()),
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let update_dto = UpdateBranchDto {
            name: Some("Updated Name".to_string()),
            description: None,
        };

        let result = BranchService::update_branch(&pool, branch.id, school_id, update_dto).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(
            updated.description,
            Some("Original Description".to_string())
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "To Be Deleted".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let result = BranchService::delete_branch(&pool, branch.id, school_id).await;

        assert!(result.is_ok());

        let fetch_result = BranchService::get_branch_by_id(&pool, branch.id, school_id).await;
        assert!(fetch_result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_branch_not_found(pool: PgPool) {
        let (school_id, _, _) = setup_test_data(&pool).await;
        let non_existent_id = Uuid::new_v4();

        let result = BranchService::delete_branch(&pool, non_existent_id, school_id).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_assign_students_to_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student1_id = create_student(&pool, school_id).await;
        let student2_id = create_student(&pool, school_id).await;

        let assign_dto = AssignStudentsToBranchDto {
            student_ids: vec![student1_id, student2_id],
        };

        let result =
            BranchService::assign_students_to_branch(&pool, branch.id, school_id, assign_dto).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.assigned_count, 2);
        assert_eq!(response.failed_ids.len(), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_assign_students_with_invalid_ids(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let valid_student_id = create_student(&pool, school_id).await;
        let invalid_student_id = Uuid::new_v4();

        let assign_dto = AssignStudentsToBranchDto {
            student_ids: vec![valid_student_id, invalid_student_id],
        };

        let result =
            BranchService::assign_students_to_branch(&pool, branch.id, school_id, assign_dto).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.assigned_count, 1);
        assert_eq!(response.failed_ids.len(), 1);
        assert_eq!(response.failed_ids[0], invalid_student_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_move_student_to_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Target Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student_id = create_student(&pool, school_id).await;

        let move_dto = MoveStudentToBranchDto {
            branch_id: Some(branch.id),
        };

        let result =
            BranchService::move_student_to_branch(&pool, student_id, school_id, move_dto).await;

        assert!(result.is_ok());

        let student_branch =
            sqlx::query_scalar!("SELECT branch_id FROM users WHERE id = $1", student_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(student_branch, Some(branch.id));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_move_student_to_null_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Initial Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student_id = create_student(&pool, school_id).await;

        sqlx::query!(
            "UPDATE users SET branch_id = $1 WHERE id = $2",
            branch.id,
            student_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let move_dto = MoveStudentToBranchDto { branch_id: None };

        let result =
            BranchService::move_student_to_branch(&pool, student_id, school_id, move_dto).await;

        assert!(result.is_ok());

        let student_branch =
            sqlx::query_scalar!("SELECT branch_id FROM users WHERE id = $1", student_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(student_branch.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_remove_student_from_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student_id = create_student(&pool, school_id).await;

        sqlx::query!(
            "UPDATE users SET branch_id = $1 WHERE id = $2",
            branch.id,
            student_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = BranchService::remove_student_from_branch(&pool, student_id, school_id).await;

        assert!(result.is_ok());

        let student_branch =
            sqlx::query_scalar!("SELECT branch_id FROM users WHERE id = $1", student_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(student_branch.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_students_in_branch(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student1_id = create_student(&pool, school_id).await;
        let student2_id = create_student(&pool, school_id).await;

        sqlx::query!(
            "UPDATE users SET branch_id = $1 WHERE id = ANY($2)",
            branch.id,
            &[student1_id, student2_id]
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = BranchService::get_students_in_branch(&pool, branch.id, school_id).await;

        assert!(result.is_ok());
        let students = result.unwrap();
        assert_eq!(students.len(), 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_branch_student_count(pool: PgPool) {
        let (school_id, level_id, _) = setup_test_data(&pool).await;

        let dto = CreateBranchDto {
            name: "Test Branch".to_string(),
            description: None,
        };
        let branch = BranchService::create_branch(&pool, level_id, school_id, dto)
            .await
            .unwrap();

        let student1_id = create_student(&pool, school_id).await;
        let student2_id = create_student(&pool, school_id).await;
        let student3_id = create_student(&pool, school_id).await;

        sqlx::query!(
            "UPDATE users SET branch_id = $1 WHERE id = ANY($2)",
            branch.id,
            &[student1_id, student2_id, student3_id]
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = BranchService::get_branch_by_id(&pool, branch.id, school_id).await;

        assert!(result.is_ok());
        let branch_with_stats = result.unwrap();
        assert_eq!(branch_with_stats.student_count, 3);
    }
}
